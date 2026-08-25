//! The open windows, as a taskbar sees them.
//!
//! `EnumWindows` returns every top-level window in the session, the vast
//! majority of which nobody should ever see: message-only windows, tool
//! windows, invisible helpers, and — the one that catches everybody — every
//! UWP application on *every other virtual desktop*. Those last are visible
//! and un-owned and look entirely legitimate; the only thing separating them
//! is `DWMWA_CLOAKED`, which is why it is checked here and why the dock would
//! otherwise list a dozen programs the user cannot see.
//!
//! Activation is the other sharp edge. `SetForegroundWindow` is ignored when
//! the calling process is not already in the foreground — deliberately, to
//! stop background programs stealing focus. Attaching to the foreground
//! thread's input queue lifts the restriction for the duration of the call.
//! It can still be refused, so the caller has a fallback rather than a
//! silently dead icon.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use bw_core::dock::WindowInfo;
use windows::Win32::Foundation::{BOOL, HWND, LPARAM, WPARAM};
use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED};
use windows::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
use windows::Win32::UI::Accessibility::{SetWinEventHook, UnhookWinEvent, HWINEVENTHOOK};
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, EnumWindows, FlashWindowEx, GetForegroundWindow, GetMessageW, GetWindow,
    GetWindowLongPtrW, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId, IsIconic,
    IsWindowVisible, PostThreadMessageW, SetForegroundWindow, ShowWindow, TranslateMessage,
    EVENT_OBJECT_DESTROY, EVENT_OBJECT_NAMECHANGE, EVENT_SYSTEM_FOREGROUND, FLASHWINFO, FLASHW_ALL,
    FLASHW_TIMERNOFG, GWL_EXSTYLE, GW_OWNER, MSG, SW_MINIMIZE, SW_RESTORE, WINEVENT_OUTOFCONTEXT,
    WINEVENT_SKIPOWNPROCESS, WM_QUIT, WS_EX_TOOLWINDOW,
};

use crate::platform::appicon;

/// Every window the dock should show, in z-order.
pub fn list() -> Vec<WindowInfo> {
    let mut found: Vec<HWND> = Vec::new();

    unsafe extern "system" fn collect(window: HWND, data: LPARAM) -> BOOL {
        let list = &mut *(data.0 as *mut Vec<HWND>);
        list.push(window);
        true.into()
    }

    unsafe {
        let _ = EnumWindows(
            Some(collect),
            LPARAM(std::ptr::addr_of_mut!(found) as isize),
        );
    }

    let foreground = unsafe { GetForegroundWindow() };

    found
        .into_iter()
        .filter(|window| unsafe { is_taskbar_window(*window) })
        .filter_map(|window| unsafe { describe(window, foreground) })
        .collect()
}

/// Whether a window is one a taskbar would list.
///
/// The rules are Explorer's, arrived at by long tradition rather than by any
/// documented contract: visible, no owner, not a tool window, and not cloaked.
unsafe fn is_taskbar_window(window: HWND) -> bool {
    if !IsWindowVisible(window).as_bool() {
        return false;
    }

    // An owned window is a dialog or a palette belonging to something else,
    // which already has its own icon.
    if !GetWindow(window, GW_OWNER).unwrap_or_default().is_invalid() {
        return false;
    }

    let styles = GetWindowLongPtrW(window, GWL_EXSTYLE) as u32;
    if styles & WS_EX_TOOLWINDOW.0 != 0 {
        return false;
    }

    // A window with no title is a helper of some kind; Explorer hides these
    // too, and they have nothing to label an icon with.
    if GetWindowTextLengthW(window) == 0 {
        return false;
    }

    !is_cloaked(window)
}

/// Whether DWM is hiding this window.
///
/// True for a UWP application suspended in the background and for anything on
/// another virtual desktop. Without this check the dock lists every store app
/// installed on the machine, all of them looking perfectly ordinary.
unsafe fn is_cloaked(window: HWND) -> bool {
    let mut cloaked: u32 = 0;
    let result = DwmGetWindowAttribute(
        window,
        DWMWA_CLOAKED,
        std::ptr::addr_of_mut!(cloaked).cast(),
        std::mem::size_of::<u32>() as u32,
    );
    // A failure means DWM has no opinion, which is not the same as cloaked.
    result.is_ok() && cloaked != 0
}

unsafe fn describe(window: HWND, foreground: HWND) -> Option<WindowInfo> {
    let mut process_id = 0u32;
    GetWindowThreadProcessId(window, Some(&mut process_id));
    if process_id == 0 {
        return None;
    }

    let (name, icon) = appicon::describe_process(process_id);
    let executable = appicon::executable_for(process_id)?;

    Some(WindowInfo {
        id: format!("{:#x}", window.0 as usize),
        title: window_title(window),
        executable,
        name,
        icon,
        active: window == foreground,
    })
}

unsafe fn window_title(window: HWND) -> String {
    let length = GetWindowTextLengthW(window);
    if length <= 0 {
        return String::new();
    }
    // One extra for the terminator GetWindowTextW writes.
    let mut buffer = vec![0u16; length as usize + 1];
    let written = GetWindowTextW(window, &mut buffer);
    String::from_utf16_lossy(&buffer[..written as usize])
}

/// Parses the id the frontend hands back.
pub fn parse_id(id: &str) -> Option<HWND> {
    let digits = id.strip_prefix("0x").unwrap_or(id);
    let value = usize::from_str_radix(digits, 16).ok()?;
    (value != 0).then_some(HWND(value as *mut std::ffi::c_void))
}

/// Brings a window to the front, restoring it if it was minimised.
///
/// Returns whether it worked. Windows can refuse — the foreground lock is
/// there to stop background processes stealing focus, and attaching to the
/// foreground thread's input queue only lifts it most of the time — so the
/// caller falls back to flashing the window rather than doing nothing.
pub fn activate(window: HWND) -> bool {
    unsafe {
        if IsIconic(window).as_bool() {
            let _ = ShowWindow(window, SW_RESTORE);
        }

        if SetForegroundWindow(window).as_bool() {
            return true;
        }

        // Refused. Attach to whatever currently holds the foreground so that
        // Windows treats this as that thread's own request.
        let foreground = GetForegroundWindow();
        if foreground.is_invalid() {
            return false;
        }
        let target = GetWindowThreadProcessId(foreground, None);
        let ours = GetCurrentThreadId();
        if target == 0 || target == ours {
            return false;
        }

        let attached = AttachThreadInput(ours, target, true).as_bool();
        let raised = SetForegroundWindow(window).as_bool();
        if attached {
            // Detaching matters: a stale attachment ties this process's input
            // queue to another's, and both stop responding properly.
            let _ = AttachThreadInput(ours, target, false);
        }
        raised
    }
}

/// Flashes a window in place of raising it.
///
/// What Explorer does when an application asks for attention it is not allowed
/// to take — which is exactly the situation when activation is refused.
pub fn flash(window: HWND) {
    let mut info = FLASHWINFO {
        cbSize: std::mem::size_of::<FLASHWINFO>() as u32,
        hwnd: window,
        dwFlags: FLASHW_ALL | FLASHW_TIMERNOFG,
        uCount: 3,
        dwTimeout: 0,
    };
    unsafe {
        let _ = FlashWindowEx(std::ptr::addr_of_mut!(info));
    }
}

pub fn minimise(window: HWND) {
    unsafe {
        let _ = ShowWindow(window, SW_MINIMIZE);
    }
}

pub fn is_minimised(window: HWND) -> bool {
    unsafe { IsIconic(window).as_bool() }
}

/// Calls back whenever the set of windows might have changed.
///
/// A timer would do, but badly: an icon that lingers for a second after its
/// application closes is the thing that makes a dock feel broken. `WinEvent`
/// hooks need a thread with a message loop, so this owns one.
pub struct WindowWatcher {
    thread: Option<std::thread::JoinHandle<()>>,
    thread_id: Arc<std::sync::atomic::AtomicU32>,
    running: Arc<AtomicBool>,
}

/// The callback, reachable from the hook's `extern "system"` function.
///
/// A `static` rather than a field because the hook signature carries no user
/// data — the same constraint the tray code works under.
static CHANGED: std::sync::OnceLock<Box<dyn Fn() + Send + Sync>> = std::sync::OnceLock::new();

impl WindowWatcher {
    /// Starts watching. The first caller's `on_change` is the one that is
    /// used; the hook API gives no way to carry per-hook state.
    pub fn new(on_change: impl Fn() + Send + Sync + 'static) -> Self {
        let _ = CHANGED.set(Box::new(on_change));

        let running = Arc::new(AtomicBool::new(true));
        let thread_id = Arc::new(std::sync::atomic::AtomicU32::new(0));

        let thread = {
            let running = running.clone();
            let thread_id = thread_id.clone();
            std::thread::Builder::new()
                .name("bw-window-watch".to_owned())
                .spawn(move || pump(&running, &thread_id))
                .ok()
        };

        Self {
            thread,
            thread_id,
            running,
        }
    }
}

impl Drop for WindowWatcher {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        // The loop is blocked in GetMessageW; post it something so it wakes up
        // and notices, rather than hanging the join forever.
        let id = self.thread_id.load(Ordering::Relaxed);
        if id != 0 {
            unsafe {
                let _ = PostThreadMessageW(id, WM_QUIT, WPARAM(0), LPARAM(0));
            }
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn pump(running: &AtomicBool, thread_id: &std::sync::atomic::AtomicU32) {
    unsafe {
        thread_id.store(GetCurrentThreadId(), Ordering::Relaxed);

        unsafe extern "system" fn on_event(
            _hook: HWINEVENTHOOK,
            _event: u32,
            _window: HWND,
            object: i32,
            child: i32,
            _thread: u32,
            _time: u32,
        ) {
            // Only the window itself, not its scrollbars, menus and captions —
            // OBJID_WINDOW is 0, and a child id means a part of a window.
            if object != 0 || child != 0 {
                return;
            }
            if let Some(changed) = CHANGED.get() {
                changed();
            }
        }

        // Create is deliberately absent: a window is created before it has a
        // title or its final styles, so it would be filtered out and the dock
        // would miss it. A name change fires once it is real, and covers the
        // title updates the dock shows anyway.
        let hooks = [
            (EVENT_SYSTEM_FOREGROUND, EVENT_SYSTEM_FOREGROUND),
            (EVENT_OBJECT_DESTROY, EVENT_OBJECT_DESTROY),
            (EVENT_OBJECT_NAMECHANGE, EVENT_OBJECT_NAMECHANGE),
        ]
        .map(|(low, high)| {
            SetWinEventHook(
                low,
                high,
                None,
                Some(on_event),
                0,
                0,
                WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
            )
        });

        let mut message = MSG::default();
        while running.load(Ordering::Relaxed) {
            // Hooks are delivered as messages, so this loop is the delivery
            // mechanism rather than idle waiting.
            if !GetMessageW(&mut message, None, 0, 0).as_bool() {
                break;
            }
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }

        for hook in hooks {
            if !hook.is_invalid() {
                let _ = UnhookWinEvent(hook);
            }
        }
    }
}
