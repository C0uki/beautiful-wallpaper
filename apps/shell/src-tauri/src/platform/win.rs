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

use std::sync::atomic::{AtomicIsize, Ordering};

use windows::core::{w, Result, PCWSTR};
use windows::Win32::Foundation::{BOOL, HWND, LPARAM, RECT, WPARAM};
use windows::Win32::Graphics::Dwm::{
    DwmSetWindowAttribute, DWMWA_SYSTEMBACKDROP_TYPE, DWMWA_USE_IMMERSIVE_DARK_MODE,
    DWMWINDOWATTRIBUTE, DWM_SYSTEMBACKDROP_TYPE,
};
use windows::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFO, MONITORINFOEXW,
};
use windows::Win32::UI::Shell::{
    SHAppBarMessage, ABE_BOTTOM, ABE_LEFT, ABE_RIGHT, ABE_TOP, ABM_NEW, ABM_QUERYPOS, ABM_REMOVE,
    ABM_SETPOS, APPBARDATA,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, FindWindowExW, FindWindowW, GetWindowLongPtrW, SendMessageTimeoutW, SetParent,
    SetWindowLongPtrW, SetWindowPos, GWL_EXSTYLE, HWND_BOTTOM, HWND_TOPMOST, SMTO_NORMAL,
    SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, WINDOW_EX_STYLE, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
    WS_EX_TRANSPARENT,
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
            // Reparenting is what puts the window *under* the icons: WorkerW is
            // the window the shell paints the wallpaper into, and it sits below
            // the icon list view.
            SetParent(hwnd, worker)?;
            SetWindowPos(
                hwnd,
                HWND_BOTTOM,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            )?;
        }
        Layer::Normal => {}
        Layer::Overlay => {
            add_ex_style(hwnd, WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE);
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

/// Makes a window ignore the mouse, so clicks land on whatever is beneath it.
///
/// This is the Windows answer to `mask: Region` — surfaces that cover the screen
/// but should only be interactive over their visible content.
///
/// # Safety
/// `hwnd` must be a live window owned by this process.
pub unsafe fn set_click_through(hwnd: HWND, click_through: bool) {
    if click_through {
        add_ex_style(hwnd, WS_EX_TRANSPARENT);
    } else {
        remove_ex_style(hwnd, WS_EX_TRANSPARENT);
    }
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

/// A registered app bar, which keeps its reserved edge until dropped.
pub struct AppBar {
    hwnd: HWND,
    callback_message: u32,
}

// SAFETY: an HWND is just a handle; the app bar is only ever used from the
// thread that owns the window, which Tauri guarantees for window operations.
unsafe impl Send for AppBar {}

impl AppBar {
    /// Reserves `thickness` pixels along `edge` for this window.
    ///
    /// Windows may hand back a different rectangle than the one asked for — when
    /// another app bar already owns part of the edge — so the granted rectangle
    /// is what gets returned and the window must move to it.
    ///
    /// # Safety
    /// `hwnd` must be a live top-level window owned by this process.
    pub unsafe fn register(
        hwnd: HWND,
        edge: Edge,
        thickness: i32,
        monitor: &Monitor,
    ) -> Option<RECT> {
        const CALLBACK: u32 = 0x0400 + 0xB0; // WM_APP + an offset of our choosing.

        let mut data = APPBARDATA {
            cbSize: std::mem::size_of::<APPBARDATA>() as u32,
            hWnd: hwnd,
            uCallbackMessage: CALLBACK,
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
        // SETPOS commits whatever came back.
        SHAppBarMessage(ABM_QUERYPOS, &mut data);
        SHAppBarMessage(ABM_SETPOS, &mut data);

        Some(data.rc)
    }
}

impl Drop for AppBar {
    fn drop(&mut self) {
        let mut data = APPBARDATA {
            cbSize: std::mem::size_of::<APPBARDATA>() as u32,
            hWnd: self.hwnd,
            uCallbackMessage: self.callback_message,
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

unsafe fn remove_ex_style(hwnd: HWND, style: WINDOW_EX_STYLE) {
    let current = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
    SetWindowLongPtrW(hwnd, GWL_EXSTYLE, current & !(style.0 as isize));
}
