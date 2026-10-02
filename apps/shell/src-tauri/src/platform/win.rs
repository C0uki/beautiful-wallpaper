//! The Win32 half of the shell.
//!
//! wlr-layer-shell gives Quickshell four things this needs: a layer to sit on, an
//! edge to anchor to, an exclusive zone, and an input mask. Windows has no such
//! protocol, so each is built from a different API:
//!
//! * the wallpaper layer  → reparenting under `WorkerW`
//! * a reserved edge      → `SHAppBarMessage`
//! * an overlay layer     → a topmost tool window that never takes focus
//! * click-through        → `WS_EX_TRANSPARENT`

use std::sync::atomic::{AtomicIsize, AtomicU64, Ordering};

use windows::core::{w, Result, PCWSTR};
use windows::Win32::Foundation::{BOOL, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Dwm::{
    DwmSetWindowAttribute, DWMWA_SYSTEMBACKDROP_TYPE, DWMWA_USE_IMMERSIVE_DARK_MODE,
    DWMWINDOWATTRIBUTE, DWM_SYSTEMBACKDROP_TYPE,
};
use windows::Win32::Graphics::Gdi::{
    CreateRectRgn, DeleteObject, EnumDisplayMonitors, GetMonitorInfoW, GetWindowRgn,
    MonitorFromWindow, ScreenToClient, HDC, HGDIOBJ, HMONITOR, MONITORINFO, MONITORINFOEXW,
    MONITOR_DEFAULTTONEAREST, RGN_ERROR,
};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::Accessibility::{SetWinEventHook, UnhookWinEvent, HWINEVENTHOOK};
use windows::Win32::UI::Shell::{
    DefSubclassProc, RemoveWindowSubclass, SHAppBarMessage, SetWindowSubclass, ABE_BOTTOM,
    ABE_LEFT, ABE_RIGHT, ABE_TOP, ABM_GETSTATE, ABM_NEW, ABM_QUERYPOS, ABM_REMOVE, ABM_SETPOS,
    ABM_SETSTATE, ABS_AUTOHIDE, APPBARDATA,
};
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, EnumWindows, FindWindowExW, FindWindowW, GetClassNameW, GetForegroundWindow,
    GetMessageW, GetWindowLongPtrW, GetWindowRect, GetWindowTextW, GetWindowThreadProcessId,
    IsWindowVisible, PostThreadMessageW, RegisterWindowMessageW, SendMessageTimeoutW,
    SendNotifyMessageW, SetParent, SetWindowLongPtrW, SetWindowPos, ShowWindow, EVENT_OBJECT_SHOW,
    GWL_EXSTYLE, HWND_BOTTOM, HWND_BROADCAST, HWND_TOPMOST, MSG, SMTO_NORMAL, STYLESTRUCT,
    SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SW_HIDE, SW_SHOW, WINDOWPOS,
    WINDOW_EX_STYLE, WINEVENT_OUTOFCONTEXT, WM_NCDESTROY, WM_QUIT, WM_STYLECHANGING,
    WM_WINDOWPOSCHANGING, WS_EX_APPWINDOW, WS_EX_LAYERED, WS_EX_TOOLWINDOW, WS_EX_TRANSPARENT,
};

/// Where a surface sits relative to the desktop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layer {
    /// Under the desktop icons, above the wallpaper. The background surface.
    Wallpaper,
    /// An ordinary window, above the desktop but below other windows.
    Normal,
    /// Always on top, never focused. Bars, popups, OSDs.
    Overlay,
}

/// Which screen edge a bar reserves space along.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edge {
    Top,
    Bottom,
    Left,
    Right,
}

impl Edge {
    fn as_abe(self) -> u32 {
        match self {
            Edge::Top => ABE_TOP,
            Edge::Bottom => ABE_BOTTOM,
            Edge::Left => ABE_LEFT,
            Edge::Right => ABE_RIGHT,
        }
    }
}

/// Backdrop material behind a translucent surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backdrop {
    None,
    /// Windows 11 only; falls back to plain on Windows 10.
    Mica,
    Acrylic,
}

/// A monitor, in the shape the frontend positions surfaces with.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Monitor {
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    /// The work area, i.e. minus the taskbar and any registered app bars.
    pub work_width: i32,
    pub work_height: i32,
    pub primary: bool,
}

/// Places a window on a layer.
///
/// # Safety
/// `hwnd` must be a live top-level window owned by this process.
pub unsafe fn set_layer(hwnd: HWND, layer: Layer) -> Result<()> {
    match layer {
        Layer::Wallpaper => {
            let worker = worker_w()?;
            // Where it is on screen now, before it becomes a child and its
            // position starts counting from the desktop window's corner.
            let mut bounds = RECT::default();
            GetWindowRect(hwnd, &mut bounds)?;
            // Reparenting is what puts the window *under* the icons: WorkerW is
            // the window the shell paints the wallpaper into, and it sits below
            // the icon list view.
            SetParent(hwnd, worker)?;
            // That window spans every monitor, so its corner is only the
            // primary monitor's when nothing sits above or left of it. With a
            // second monitor above, the wallpaper went to the top of that one
            // and left the primary monitor all but uncovered.
            let mut corner = POINT {
                x: bounds.left,
                y: bounds.top,
            };
            let _ = ScreenToClient(worker, &mut corner);
            SetWindowPos(
                hwnd,
                HWND_BOTTOM,
                corner.x,
                corner.y,
                0,
                0,
                SWP_NOSIZE | SWP_NOACTIVATE,
            )?;
        }
        Layer::Normal => {}
        Layer::Overlay => {
            // `WS_EX_NOACTIVATE` is Tao's to set, through `focusable`:
            // anything added here is erased the next time Tao writes the
            // styles, and it writes them whole, from its own flags.
            //
            // `WS_EX_TOOLWINDOW` has no flag of its own over there, so it is
            // set here for the window as it stands and defended by a subclass
            // for every write to come.
            add_ex_style(hwnd, WS_EX_TOOLWINDOW);
            let _ = SetWindowSubclass(hwnd, Some(keep_tool_window), KEEP_TOOL_WINDOW, 0);
            SetWindowPos(
                hwnd,
                HWND_TOPMOST,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            )?;
        }
    }
    Ok(())
}

/// Tells our subclass apart from anyone else's on the same window.
const KEEP_TOOL_WINDOW: usize = 0x6277_7477; // "bwtw"

/// Puts `WS_EX_TOOLWINDOW` back into every write of a surface's ex-styles.
///
/// Tao rewrites `GWL_EXSTYLE` whole, from its own `WindowFlags`, whenever it
/// shows or hides a window — and it has no flag for `WS_EX_TOOLWINDOW`, so the
/// bit is gone by the first `show()` after it was set. Without it a surface is
/// an ordinary application window: `skip_taskbar` keeps it off the taskbar
/// through `ITaskbarList`, but nothing keeps it out of Alt-Tab, and the toasts
/// sit there visible, parked off the edge of the screen, for the life of the
/// shell.
///
/// Windows asks a window before it writes its styles, through
/// `WM_STYLECHANGING`, and a subclass may edit what is about to be written.
/// That is the one point every writer passes through, so the bit goes back
/// here rather than after each of Tao's calls — chasing those is what let this
/// through twice already.
unsafe extern "system" fn keep_tool_window(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _id: usize,
    _data: usize,
) -> LRESULT {
    if msg == WM_STYLECHANGING && wparam.0 as i32 == GWL_EXSTYLE.0 {
        let styles = lparam.0 as *mut STYLESTRUCT;
        if !styles.is_null() {
            (*styles).styleNew |= WS_EX_TOOLWINDOW.0;
        }
    }

    // The window outlives nothing, so the subclass comes off with it.
    if msg == WM_NCDESTROY {
        let _ = RemoveWindowSubclass(hwnd, Some(keep_tool_window), KEEP_TOOL_WINDOW);
    }

    DefSubclassProc(hwnd, msg, wparam, lparam)
}

const EDGE_WINDOW: usize = 0x6277_6577; // "bwew"

/// The hot corners' window, once it exists.
static HOT_CORNERS: AtomicIsize = AtomicIsize::new(0);

/// What to do when Explorer comes back: see [`watch_edge_window`].
static TASKBAR_CREATED: std::sync::OnceLock<Box<dyn Fn() + Send + Sync>> =
    std::sync::OnceLock::new();

/// Names the hot corners' window, for [`watch_edge_window`] to keep others under.
pub fn set_hot_corners(hwnd: HWND) {
    HOT_CORNERS.store(hwnd.0 as isize, Ordering::Relaxed);
}

/// Looks after a window that runs along an edge of the screen: the bar or the
/// dock.
///
/// Two things happen to those that nothing else would notice:
///
///   * a click raises them — WebView2 brings its window to the front even when
///     it cannot be activated — over the hot corners, whose strips they cover.
///     A bar clicked once left the top corners dead until the shell
///     restarted. Every change of their place in the z-order is redirected to
///     just under the hot corners, which is the one point all of them pass.
///   * Explorer restarting forgets every app bar, so the edge the bar held was
///     given back while the bar went on sitting in it. Explorer announces
///     itself with `TaskbarCreated`, sent to every top-level window, and
///     `on_taskbar_created` runs then. The first caller's is the one kept.
///
/// # Safety
/// `hwnd` must be a live top-level window owned by this process.
pub unsafe fn watch_edge_window(hwnd: HWND, on_taskbar_created: impl Fn() + Send + Sync + 'static) {
    let _ = TASKBAR_CREATED.set(Box::new(on_taskbar_created));
    let _ = SetWindowSubclass(hwnd, Some(edge_window), EDGE_WINDOW, 0);
}

pub fn taskbar_created() -> u32 {
    static MESSAGE: std::sync::OnceLock<u32> = std::sync::OnceLock::new();
    *MESSAGE.get_or_init(|| unsafe { RegisterWindowMessageW(w!("TaskbarCreated")) })
}

/// When the shell last sent `TaskbarCreated` itself, in milliseconds since it
/// first asked; zero for never.
static ANNOUNCED: AtomicU64 = AtomicU64::new(0);

fn milliseconds() -> u64 {
    static START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
    START
        .get_or_init(std::time::Instant::now)
        .elapsed()
        .as_millis() as u64
        + 1
}

/// Asks every application to add its notification-area icons again, the
/// way Explorer does when it starts: the shell hosts them, and the ones
/// added before it did went only to Explorer.
pub fn announce_taskbar() {
    ANNOUNCED.store(milliseconds(), Ordering::Relaxed);
    unsafe {
        let _ = SendNotifyMessageW(HWND_BROADCAST, taskbar_created(), WPARAM(0), LPARAM(0));
    }
}

/// Whether a `TaskbarCreated` just heard is the shell's own rather than a
/// new Explorer's. Its own reaches every window it has, the bar's and the
/// dock's included, and is no reason to put them back in their places.
pub fn announced_by_us() -> bool {
    let at = ANNOUNCED.load(Ordering::Relaxed);
    at != 0 && milliseconds().saturating_sub(at) < 3000
}

/// Explorer's taskbar: the first `Shell_TrayWnd` that is not the shell's own
/// notification-area host, which sits in front of it on purpose.
pub fn explorer_tray() -> Option<HWND> {
    let ours = std::process::id();
    let mut previous = HWND::default();
    unsafe {
        loop {
            let found = FindWindowExW(
                HWND::default(),
                previous,
                w!("Shell_TrayWnd"),
                PCWSTR::null(),
            )
            .ok()?;
            if found.0.is_null() {
                return None;
            }
            let mut process = 0u32;
            GetWindowThreadProcessId(found, Some(&mut process));
            if process != ours {
                return Some(found);
            }
            previous = found;
        }
    }
}

unsafe extern "system" fn edge_window(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _id: usize,
    _data: usize,
) -> LRESULT {
    // The shell's own `TaskbarCreated` asks applications for their icons; it
    // says nothing about Explorer, and nothing here needs redoing.
    if msg == taskbar_created() && announced_by_us() {
        return DefSubclassProc(hwnd, msg, wparam, lparam);
    }

    if msg == WM_WINDOWPOSCHANGING {
        let position = lparam.0 as *mut WINDOWPOS;
        let corners = HWND(HOT_CORNERS.load(Ordering::Relaxed) as _);
        if !position.is_null()
            && (*position).flags.0 & SWP_NOZORDER.0 == 0
            && !corners.0.is_null()
            && corners != hwnd
            && IsWindowVisible(corners).as_bool()
        {
            (*position).hwndInsertAfter = corners;
        }
    }

    if msg == taskbar_created() {
        if let Some(callback) = TASKBAR_CREATED.get() {
            callback();
        }
    }

    if msg == WM_NCDESTROY {
        let _ = RemoveWindowSubclass(hwnd, Some(edge_window), EDGE_WINDOW);
    }

    DefSubclassProc(hwnd, msg, wparam, lparam)
}

/// Applies a DWM backdrop and dark-mode titlebar hint.
///
/// Mica needs Windows 11 build 22621; asking for it on anything older simply
/// fails, which is why the error is swallowed rather than propagated.
///
/// # Safety
/// `hwnd` must be a live window owned by this process.
pub unsafe fn set_backdrop(hwnd: HWND, backdrop: Backdrop, dark: bool) {
    let dark_flag = BOOL::from(dark);
    let _ = DwmSetWindowAttribute(
        hwnd,
        DWMWA_USE_IMMERSIVE_DARK_MODE,
        std::ptr::addr_of!(dark_flag).cast(),
        std::mem::size_of::<BOOL>() as u32,
    );

    // DWMSBT_NONE = 1, DWMSBT_MAINWINDOW (Mica) = 2, DWMSBT_TRANSIENTWINDOW
    // (Acrylic) = 3.
    let value: i32 = match backdrop {
        Backdrop::None => 1,
        Backdrop::Mica => 2,
        Backdrop::Acrylic => 3,
    };
    let _ = DwmSetWindowAttribute(
        hwnd,
        DWMWINDOWATTRIBUTE(DWMWA_SYSTEMBACKDROP_TYPE.0),
        std::ptr::addr_of!(value).cast(),
        std::mem::size_of::<DWM_SYSTEMBACKDROP_TYPE>() as u32,
    );
}

/// WM_APP plus an offset of our choosing, for the shell's app bar callbacks.
const APPBAR_CALLBACK: u32 = 0x0400 + 0xB0;

/// A registered app bar. Reserving screen space is not a one-shot call: the
/// registration lives until it is removed, and a process that exits without
/// removing it leaves the work area permanently shrunk. Holding the registration
/// in a value with a `Drop` is what guarantees the edge is given back.
#[must_use = "dropping the AppBar immediately gives the reserved edge back"]
pub struct AppBar {
    hwnd: HWND,
    /// The rectangle Windows actually granted, which may differ from the one
    /// asked for when another app bar already owns part of the edge.
    pub granted: RECT,
}

// SAFETY: an HWND is just a handle; the app bar is only ever used from the
// thread that owns the window, which Tauri guarantees for window operations.
unsafe impl Send for AppBar {}

impl AppBar {
    /// Reserves `thickness` pixels along `edge` for this window.
    ///
    /// The caller must move the window to [`AppBar::granted`]: Windows reserves
    /// the rectangle it returns, not the one it was asked for.
    ///
    /// # Safety
    /// `hwnd` must be a live top-level window owned by this process.
    pub unsafe fn register(
        hwnd: HWND,
        edge: Edge,
        thickness: i32,
        monitor: &Monitor,
    ) -> Option<Self> {
        let mut data = APPBARDATA {
            cbSize: std::mem::size_of::<APPBARDATA>() as u32,
            hWnd: hwnd,
            uCallbackMessage: APPBAR_CALLBACK,
            uEdge: edge.as_abe(),
            rc: RECT::default(),
            lParam: LPARAM(0),
        };

        if SHAppBarMessage(ABM_NEW, &mut data) == 0 {
            return None;
        }

        data.rc = match edge {
            Edge::Top => RECT {
                left: monitor.x,
                top: monitor.y,
                right: monitor.x + monitor.width,
                bottom: monitor.y + thickness,
            },
            Edge::Bottom => RECT {
                left: monitor.x,
                top: monitor.y + monitor.height - thickness,
                right: monitor.x + monitor.width,
                bottom: monitor.y + monitor.height,
            },
            Edge::Left => RECT {
                left: monitor.x,
                top: monitor.y,
                right: monitor.x + thickness,
                bottom: monitor.y + monitor.height,
            },
            Edge::Right => RECT {
                left: monitor.x + monitor.width - thickness,
                top: monitor.y,
                right: monitor.x + monitor.width,
                bottom: monitor.y + monitor.height,
            },
        };

        // QUERYPOS lets the shell adjust the rectangle around existing bars;
        // SETPOS commits whatever came back. The shell only moves the side
        // that meets another bar, so the far side has to be put back at
        // `thickness` from it: against the taskbar along the bottom — which
        // holds its edge even while hidden — the bottom came up past the top
        // and the bar was granted a rectangle of no height at all.
        SHAppBarMessage(ABM_QUERYPOS, &mut data);
        match edge {
            Edge::Top => data.rc.bottom = data.rc.top + thickness,
            Edge::Bottom => data.rc.top = data.rc.bottom - thickness,
            Edge::Left => data.rc.right = data.rc.left + thickness,
            Edge::Right => data.rc.left = data.rc.right - thickness,
        }
        SHAppBarMessage(ABM_SETPOS, &mut data);

        Some(Self {
            hwnd,
            granted: data.rc,
        })
    }
}

impl Drop for AppBar {
    fn drop(&mut self) {
        let mut data = APPBARDATA {
            cbSize: std::mem::size_of::<APPBARDATA>() as u32,
            hWnd: self.hwnd,
            uCallbackMessage: APPBAR_CALLBACK,
            uEdge: ABE_TOP,
            rc: RECT::default(),
            lParam: LPARAM(0),
        };
        // Leaving a registered app bar behind would permanently shrink the work
        // area, even after the process exits.
        unsafe { SHAppBarMessage(ABM_REMOVE, &mut data) };
    }
}

/// Enumerates the monitors, with their work areas.
pub fn monitors() -> Vec<Monitor> {
    static FOUND: AtomicIsize = AtomicIsize::new(0);

    unsafe extern "system" fn callback(
        monitor: HMONITOR,
        _dc: HDC,
        _rect: *mut RECT,
        data: LPARAM,
    ) -> BOOL {
        let list = &mut *(data.0 as *mut Vec<Monitor>);

        let mut info = MONITORINFOEXW {
            monitorInfo: MONITORINFO {
                cbSize: std::mem::size_of::<MONITORINFOEXW>() as u32,
                ..Default::default()
            },
            ..Default::default()
        };

        if GetMonitorInfoW(monitor, std::ptr::addr_of_mut!(info).cast()).as_bool() {
            let full = info.monitorInfo.rcMonitor;
            let work = info.monitorInfo.rcWork;
            let name = String::from_utf16_lossy(
                &info
                    .szDevice
                    .iter()
                    .take_while(|c| **c != 0)
                    .copied()
                    .collect::<Vec<u16>>(),
            );
            list.push(Monitor {
                name,
                x: full.left,
                y: full.top,
                width: full.right - full.left,
                height: full.bottom - full.top,
                work_width: work.right - work.left,
                work_height: work.bottom - work.top,
                // MONITORINFOF_PRIMARY
                primary: info.monitorInfo.dwFlags & 1 != 0,
            });
        }
        BOOL(1)
    }

    let mut list: Vec<Monitor> = Vec::new();
    let _ = FOUND.load(Ordering::Relaxed);
    unsafe {
        let _ = EnumDisplayMonitors(
            HDC::default(),
            None,
            Some(callback),
            LPARAM(std::ptr::addr_of_mut!(list) as isize),
        );
    }
    list
}

/// Finds the `WorkerW` window that sits behind the desktop icons.
///
/// `Progman` only creates it after being sent the undocumented `0x052C`
/// message — the standard trick every animated-wallpaper tool on Windows uses.
/// Once created, the right `WorkerW` is the sibling of the `SHELLDLL_DefView`
/// that hosts the icons.
fn worker_w() -> Result<HWND> {
    unsafe {
        let progman = FindWindowW(w!("Progman"), PCWSTR::null())?;
        // Ask Progman to spawn the WorkerW layer. It ignores the result, so a
        // timeout here is not an error.
        let mut ignored = 0usize;
        SendMessageTimeoutW(
            progman,
            0x052C,
            WPARAM(0),
            LPARAM(0),
            SMTO_NORMAL,
            1000,
            Some(std::ptr::addr_of_mut!(ignored)),
        );

        struct Search {
            found: HWND,
        }

        unsafe extern "system" fn enumerate(hwnd: HWND, data: LPARAM) -> BOOL {
            let search = &mut *(data.0 as *mut Search);
            // The WorkerW we want is the one immediately after the window that
            // hosts SHELLDLL_DefView.
            if FindWindowExW(
                hwnd,
                HWND::default(),
                w!("SHELLDLL_DefView"),
                PCWSTR::null(),
            )
            .is_ok()
            {
                if let Ok(worker) =
                    FindWindowExW(HWND::default(), hwnd, w!("WorkerW"), PCWSTR::null())
                {
                    search.found = worker;
                    return BOOL(0);
                }
            }
            BOOL(1)
        }

        let mut search = Search {
            found: HWND::default(),
        };
        let _ = EnumWindows(
            Some(enumerate),
            LPARAM(std::ptr::addr_of_mut!(search) as isize),
        );

        // Some Windows builds keep the wallpaper in Progman itself; parenting
        // there still renders below the icons.
        Ok(if search.found.0.is_null() {
            progman
        } else {
            search.found
        })
    }
}

unsafe fn add_ex_style(hwnd: HWND, style: WINDOW_EX_STYLE) {
    let current = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
    SetWindowLongPtrW(hwnd, GWL_EXSTYLE, current | style.0 as isize);
}

/// Whether Explorer's desktop window, the one the wallpaper surface goes
/// into, exists.
pub fn desktop_exists() -> bool {
    unsafe { FindWindowW(w!("Progman"), PCWSTR::null()).is_ok() }
}

/// The window the user is currently working in.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActiveWindow {
    pub title: String,
    /// The window class, which is the closest Windows has to an app id.
    pub class: String,
    /// Whether it covers its whole monitor, so the shell's decorations can
    /// get out of the way of a film.
    pub fullscreen: bool,
}

/// Reads the foreground window's title and class.
///
/// Returns an empty reading when the desktop itself has focus, which is a normal
/// state rather than a failure.
pub fn active_window() -> ActiveWindow {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.0.is_null() {
            return ActiveWindow::default();
        }

        let mut title = [0u16; 512];
        let written = GetWindowTextW(hwnd, &mut title);
        let mut class = [0u16; 256];
        let class_written = GetClassNameW(hwnd, &mut class);

        let class = String::from_utf16_lossy(&class[..class_written.max(0) as usize]);
        ActiveWindow {
            title: String::from_utf16_lossy(&title[..written.max(0) as usize]),
            fullscreen: is_fullscreen(hwnd, &class),
            class,
        }
    }
}

/// Whether this window covers its whole monitor.
///
/// There is no flag to read — Windows has no concept of a full-screen window,
/// only of a window that happens to be the size of the screen — so this
/// compares rectangles, which is also what actually matters visually.
///
/// The shell's own furniture is excluded by class. Explorer's desktop spans
/// the monitor by definition and the taskbar's parent does too, so without
/// this the corners would vanish the moment the user clicked the wallpaper.
unsafe fn is_fullscreen(hwnd: HWND, class: &str) -> bool {
    if matches!(
        class,
        "Progman" | "WorkerW" | "Shell_TrayWnd" | "Shell_SecondaryTrayWnd" | "SysListView32"
    ) {
        return false;
    }

    let mut bounds = RECT::default();
    if GetWindowRect(hwnd, &mut bounds).is_err() {
        return false;
    }

    let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
    if monitor.is_invalid() {
        return false;
    }
    let mut info = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    if !GetMonitorInfoW(monitor, &mut info).as_bool() {
        return false;
    }

    // Exactly, not approximately: a maximised window stops at the work area
    // and a full-screen one does not, and the difference between them is the
    // taskbar. Treating "close enough" as full screen would hide the corners
    // for every maximised window on the machine.
    let screen = info.rcMonitor;
    bounds.left <= screen.left
        && bounds.top <= screen.top
        && bounds.right >= screen.right
        && bounds.bottom >= screen.bottom
}

/// Whether this window covers a whole monitor with no way for a click to
/// reach past it.
///
/// Every surface that spans the screen is meant to have one of two things: it
/// is click-through, or it carries a window region that cuts it back to the
/// parts that should exist. One that is visible with neither swallows every
/// click on that monitor, and because these surfaces are transparent the
/// desktop does not look covered — it looks broken.
///
/// This is a watchdog rather than a guard. It changes nothing; it names the
/// surface in the log so the next report of "the screen stopped responding"
/// arrives with the answer already in it.
///
/// # Safety
/// `hwnd` must be a live window owned by this process.
pub unsafe fn swallows_its_monitor(hwnd: HWND) -> bool {
    let mut bounds = RECT::default();
    if GetWindowRect(hwnd, &mut bounds).is_err() {
        return false;
    }

    let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
    let mut info = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    if monitor.is_invalid() || !GetMonitorInfoW(monitor, &mut info).as_bool() {
        return false;
    }

    let screen = info.rcMonitor;
    let covers = bounds.left <= screen.left
        && bounds.top <= screen.top
        && bounds.right >= screen.right
        && bounds.bottom >= screen.bottom;
    if !covers {
        return false;
    }

    // Both, not either: `WS_EX_TRANSPARENT` on its own leaves a window that
    // reads as click-through in any list of styles and still catches every
    // click. That is the shape of the bug this watchdog exists for, so it must
    // not be the shape it treats as safe.
    let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
    let lets_the_pointer_through =
        ex & WS_EX_TRANSPARENT.0 as isize != 0 && ex & WS_EX_LAYERED.0 as isize != 0;
    if lets_the_pointer_through {
        return false;
    }

    // Lost its furniture styles. A surface that spans the screen and is an
    // ordinary application window is one Alt-Tab lists and one a click can
    // bring to the front, neither of which a decoration should be. Either way
    // something has written over the styles, which is worth saying out loud.
    if ex & WS_EX_APPWINDOW.0 as isize != 0 && ex & WS_EX_TOOLWINDOW.0 as isize == 0 {
        return true;
    }

    // `GetWindowRgn` needs somewhere to put a copy of the region, and answers
    // `ERROR` (0) when the window has none. The scratch region is ours either
    // way — unlike `SetWindowRgn`, this call never takes ownership.
    let scratch = CreateRectRgn(0, 0, 1, 1);
    let has_region = GetWindowRgn(hwnd, scratch) != RGN_ERROR;
    let _ = DeleteObject(HGDIOBJ::from(scratch));

    !has_region
}

/// Shows or hides the stock taskbar.
///
/// Only ever called when the user asks for it: a shell that hides the taskbar
/// without being told, and leaves it hidden if it crashes, is a bad neighbour.
/// The secondary taskbars on other monitors are handled too.
///
/// # Safety
/// Changes global desktop state; the caller must restore it before exiting.
pub unsafe fn set_taskbar_visible(visible: bool) {
    let command = if visible { SW_SHOW } else { SW_HIDE };

    if let Some(primary) = explorer_tray() {
        let _ = ShowWindow(primary, command);
    }

    // Secondary taskbars each get their own window of this class.
    let mut previous = HWND::default();
    while let Ok(secondary) = FindWindowExW(
        HWND::default(),
        previous,
        w!("Shell_SecondaryTrayWnd"),
        PCWSTR::null(),
    ) {
        if secondary.0.is_null() {
            break;
        }
        let _ = ShowWindow(secondary, command);
        previous = secondary;
    }
}

/// Hides the taskbar for as long as it is held, and puts it back on drop.
///
/// The same shape as [`AppBar`], for the same reason: this changes the desktop
/// for every program on it, so giving it back has to be tied to something with
/// a lifetime rather than to remembering.
///
/// Hidden is not enough on its own: a hidden taskbar still holds its edge of
/// the screen, so maximised windows stopped short of a strip with nothing in
/// it and a bar along the bottom sat above it. Set to hide itself, it gives
/// the edge back — and Explorer shows it on the way, so it is hidden after.
///
/// It covers a graceful exit, which is not every exit — Windows ends a process
/// killed from Task Manager without unwinding, and the taskbar would stay
/// hidden, and set to hide itself, with no shell left to put either back.
/// `bw taskbar show` is the way back, and it works with nothing running: the
/// setting the taskbar had is kept on disk while it is held, for that.
pub struct HiddenTaskbar {
    /// The `ABS_*` flags it had, to put back.
    state: u32,
    /// Hides it again whenever Explorer shows it. Explorer does that on its
    /// own after the state changes — twice, around 300 and 400ms later, so
    /// hiding it once straight away lasted a third of a second — and a
    /// taskbar left showing while set to hide itself comes up under the
    /// pointer at the bottom of the screen. Every process's, not only the
    /// Explorer running now: a restarted Explorer makes a new taskbar.
    keeper: Option<(std::thread::JoinHandle<()>, u32)>,
}

impl HiddenTaskbar {
    /// # Safety
    /// Changes global desktop state; the returned value must be kept until the
    /// taskbar should come back.
    pub unsafe fn hide() -> Self {
        let keeper = keep_taskbar_hidden();
        // A setting left on disk is from a shell that was killed while holding
        // the taskbar, and is what it had before that one: the taskbar's own
        // setting now is the killed shell's doing, and keeping that would make
        // hiding itself permanent.
        let state = saved_taskbar_state().unwrap_or_else(|| {
            let state = taskbar_state(ABM_GETSTATE, 0) as u32;
            let file = taskbar_state_file();
            if let Some(directory) = file.parent() {
                let _ = std::fs::create_dir_all(directory);
            }
            let _ = std::fs::write(file, state.to_string());
            state
        });
        taskbar_state(ABM_SETSTATE, state | ABS_AUTOHIDE);
        set_taskbar_visible(false);
        Self { state, keeper }
    }

    /// Hides it again, set to hide itself, after Explorer has made it anew.
    ///
    /// # Safety
    /// Changes global desktop state.
    pub unsafe fn reassert(&self) {
        taskbar_state(ABM_SETSTATE, self.state | ABS_AUTOHIDE);
        set_taskbar_visible(false);
    }
}

impl Drop for HiddenTaskbar {
    fn drop(&mut self) {
        // The keeper first, or it would hide the taskbar being given back.
        if let Some((thread, id)) = self.keeper.take() {
            unsafe {
                let _ = PostThreadMessageW(id, WM_QUIT, WPARAM(0), LPARAM(0));
            }
            let _ = thread.join();
        }
        unsafe {
            taskbar_state(ABM_SETSTATE, self.state);
            set_taskbar_visible(true);
        }
        let _ = std::fs::remove_file(taskbar_state_file());
    }
}

/// Puts back the taskbar setting a killed shell left behind, if it left one.
///
/// # Safety
/// Changes global desktop state.
pub unsafe fn restore_saved_taskbar_state() {
    if let Some(state) = saved_taskbar_state() {
        taskbar_state(ABM_SETSTATE, state);
        let _ = std::fs::remove_file(taskbar_state_file());
    }
}

/// Where the taskbar's own setting is kept while the shell holds it.
fn taskbar_state_file() -> std::path::PathBuf {
    bw_core::paths::state_dir().join("taskbar-state")
}

fn saved_taskbar_state() -> Option<u32> {
    std::fs::read_to_string(taskbar_state_file())
        .ok()?
        .trim()
        .parse()
        .ok()
}

/// A thread that hides every taskbar anything shows, until it is sent
/// `WM_QUIT`. Returns it with its thread id, once the hook is in place.
fn keep_taskbar_hidden() -> Option<(std::thread::JoinHandle<()>, u32)> {
    unsafe extern "system" fn on_show(
        _hook: HWINEVENTHOOK,
        _event: u32,
        window: HWND,
        object: i32,
        _child: i32,
        _thread: u32,
        _time: u32,
    ) {
        // The window itself (`OBJID_WINDOW`), not a part of one.
        if object != 0 {
            return;
        }
        let mut class = [0u16; 32];
        let length = GetClassNameW(window, &mut class) as usize;
        let class = String::from_utf16_lossy(&class[..length]);
        if class == "Shell_TrayWnd" || class == "Shell_SecondaryTrayWnd" {
            let _ = ShowWindow(window, SW_HIDE);
        }
    }

    let (ready, id) = std::sync::mpsc::channel();
    let thread = std::thread::Builder::new()
        .name("bw-taskbar-keeper".to_owned())
        .spawn(move || unsafe {
            let hook = SetWinEventHook(
                EVENT_OBJECT_SHOW,
                EVENT_OBJECT_SHOW,
                None,
                Some(on_show),
                0,
                0,
                WINEVENT_OUTOFCONTEXT,
            );
            // Only now: the thread has a message queue for `WM_QUIT` to land
            // in, and a quit posted before it did would be lost and the join
            // would never return.
            let _ = ready.send(GetCurrentThreadId());

            let mut message = MSG::default();
            while GetMessageW(&mut message, None, 0, 0).as_bool() {
                DispatchMessageW(&message);
            }
            if !hook.is_invalid() {
                let _ = UnhookWinEvent(hook);
            }
        })
        .ok()?;
    let id = id.recv().ok()?;
    Some((thread, id))
}

/// Reads or sets the taskbar's `ABS_*` flags, which are the user's own
/// taskbar settings and shared by the taskbars on every monitor.
unsafe fn taskbar_state(message: u32, state: u32) -> usize {
    let mut data = APPBARDATA {
        cbSize: std::mem::size_of::<APPBARDATA>() as u32,
        hWnd: explorer_tray().unwrap_or_default(),
        lParam: LPARAM(state as isize),
        ..Default::default()
    };
    SHAppBarMessage(message, &mut data)
}

/// Bytes sent and received across all interfaces since boot.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NetworkCounters {
    pub received: u64,
    pub sent: u64,
}

/// Reads the interface table and sums the physical, connected interfaces.
///
/// Loopback and tunnel interfaces are excluded: counting them would double every
/// byte that never left the machine.
pub fn network_counters() -> NetworkCounters {
    use windows::Win32::NetworkManagement::IpHelper::{FreeMibTable, GetIfTable2, MIB_IF_TABLE2};
    use windows::Win32::NetworkManagement::Ndis::{IfOperStatusUp, NET_IF_CONNECTION_DEDICATED};

    let mut totals = NetworkCounters::default();
    unsafe {
        let mut table: *mut MIB_IF_TABLE2 = std::ptr::null_mut();
        if GetIfTable2(&mut table).is_err() || table.is_null() {
            return totals;
        }

        let rows =
            std::slice::from_raw_parts((*table).Table.as_ptr(), (*table).NumEntries as usize);
        for row in rows {
            // `InterfaceAndOperStatusFlags` is a packed bitfield; bit 0 is
            // HardwareInterface, which excludes the loopback, tunnel and virtual
            // adapters that would otherwise double-count local traffic.
            let hardware_interface = row.InterfaceAndOperStatusFlags._bitfield & 1 != 0;
            if row.OperStatus != IfOperStatusUp
                || row.ConnectionType != NET_IF_CONNECTION_DEDICATED
                || !hardware_interface
            {
                continue;
            }
            totals.received = totals.received.saturating_add(row.InOctets);
            totals.sent = totals.sent.saturating_add(row.OutOctets);
        }

        FreeMibTable(table.cast());
    }
    totals
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DestroyWindow, WINDOW_STYLE, WS_POPUP,
    };

    /// Tao writes a surface's ex-styles whole and has no flag for
    /// `WS_EX_TOOLWINDOW`, so every `show()` would drop the bit. Twice now a
    /// fix has put it back in one place and left another writer to erase it;
    /// this is the writer, standing in for Tao.
    #[test]
    fn a_surface_keeps_its_tool_window_bit_through_a_wholesale_rewrite() {
        unsafe {
            // `STATIC` is a class Windows has already registered for us.
            let hwnd = CreateWindowExW(
                WS_EX_APPWINDOW,
                w!("STATIC"),
                w!("tool window test"),
                WINDOW_STYLE(WS_POPUP.0),
                0,
                0,
                1,
                1,
                None,
                None,
                None,
                None,
            )
            .expect("Windows would not give us a window to test with");

            let _ = SetWindowSubclass(hwnd, Some(keep_tool_window), KEEP_TOOL_WINDOW, 0);

            // What Tao does: the whole style word, from its own flags, with no
            // `WS_EX_TOOLWINDOW` anywhere in it.
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, WS_EX_APPWINDOW.0 as isize);

            let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
            let _ = DestroyWindow(hwnd);

            assert!(
                ex & WS_EX_TOOLWINDOW.0 as isize != 0,
                "the rewrite dropped WS_EX_TOOLWINDOW: {ex:#010x}",
            );
        }
    }
}
