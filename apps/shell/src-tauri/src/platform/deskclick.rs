//! Taking the desktop's right button away from Explorer.
//!
//! The original does not have this problem: on Hyprland the shell's background
//! layer *is* the desktop, and a right-click on it is simply an event. Here the
//! background surface is reparented under `WorkerW` so it sits below the icons
//! (`win::set_layer`, `Layer::Wallpaper`), which means a click on the desktop
//! goes to Explorer's `SysListView32` and never reaches this process at all.
//! Floating the surface above the icons instead would take the click and cost
//! the icons: no selection, no drag, no double-click to open.
//!
//! So the only way to replace Explorer's menu is a system-wide low-level mouse
//! hook that swallows the button before Explorer sees it. That is a real cost,
//! which is why it lives under `hacks.desktopMenu` and is off by default:
//!
//! - **Windows removes the hook silently** if this process fails to answer
//!   within `LowLevelHooksTimeout`. Nothing is reported; the menu simply stops
//!   appearing one day. Hence the rule below that the callback does no work.
//! - **It is an API security software watches**, because it is how a keylogger
//!   would be built. A shell asking for it is unusual enough to be worth the
//!   user's explicit consent.
//!
//! The menu is reachable by its key, from the launcher and from the CLI
//! whether or not this is switched on.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, OnceLock};

use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetAncestor, GetClassNameW, GetMessageW, PostThreadMessageW,
    SetWindowsHookExW, TranslateMessage, UnhookWindowsHookEx, WindowFromPoint, GA_ROOT, MSG,
    MSLLHOOKSTRUCT, WH_MOUSE_LL, WM_QUIT, WM_RBUTTONDOWN, WM_RBUTTONUP,
};

/// Where the desktop was clicked, in physical screen pixels.
pub type Click = (i32, i32);

/// The channel the hook drops a click into.
///
/// A `static` because the hook signature carries no user data — the same
/// constraint `windows::WindowWatcher` and the tray code work under. It is
/// written once and outlives every hook: switching `hacks.desktopMenu` off and
/// on again replaces the hook, not what a click means.
static CLICKS: OnceLock<Sender<Click>> = OnceLock::new();

/// Whether the press was swallowed, so its release can be swallowed too.
///
/// Eating one half of a click leaves whoever is listening believing the button
/// is still down: Explorer starts a rubber-band selection that never ends.
static ATE_THE_PRESS: AtomicBool = AtomicBool::new(false);

/// A live `WH_MOUSE_LL` hook, and the thread pumping messages for it.
///
/// Holding this is what keeps the hook registered; dropping it unhooks and
/// stops the thread.
pub struct DesktopClickHook {
    thread: Option<std::thread::JoinHandle<()>>,
    thread_id: Arc<AtomicU32>,
    running: Arc<AtomicBool>,
}

impl DesktopClickHook {
    /// Starts watching for right-clicks on the desktop.
    ///
    /// `on_click` runs on a thread of its own, never inside the hook, and is
    /// given the point in physical screen pixels. The first caller's closure is
    /// the one that is used; the hook API gives no way to carry per-hook state.
    pub fn new(on_click: impl Fn(Click) + Send + 'static) -> Self {
        let (sender, receiver) = mpsc::channel();
        // Only the first hook gets to start the consumer; a later one finds the
        // channel already claimed and lets its own end go. Kept apart from the
        // hook thread on purpose: this one is allowed to take as long as
        // opening a window takes, and the hook is not allowed to take any time
        // at all.
        if CLICKS.set(sender).is_ok() {
            std::thread::Builder::new()
                .name("bw-desktop-click".to_owned())
                .spawn(move || {
                    for click in receiver {
                        on_click(click);
                    }
                })
                .ok();
        }

        let running = Arc::new(AtomicBool::new(true));
        let thread_id = Arc::new(AtomicU32::new(0));

        let thread = {
            let running = running.clone();
            let thread_id = thread_id.clone();
            std::thread::Builder::new()
                .name("bw-desktop-hook".to_owned())
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

impl Drop for DesktopClickHook {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        // The loop is blocked in GetMessageW; post it something so it wakes and
        // notices, rather than hanging the join forever.
        let id = self.thread_id.load(Ordering::Relaxed);
        if id != 0 {
            unsafe {
                let _ = PostThreadMessageW(id, WM_QUIT, WPARAM(0), LPARAM(0));
            }
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
        ATE_THE_PRESS.store(false, Ordering::Relaxed);
    }
}

fn pump(running: &AtomicBool, thread_id: &AtomicU32) {
    unsafe {
        thread_id.store(GetCurrentThreadId(), Ordering::Relaxed);

        let hook = match SetWindowsHookExW(WH_MOUSE_LL, Some(on_mouse), None, 0) {
            Ok(hook) => hook,
            Err(error) => {
                tracing::warn!(%error, "could not watch the desktop's mouse buttons");
                return;
            }
        };

        let mut message = MSG::default();
        while running.load(Ordering::Relaxed) {
            // A low-level hook is delivered on the thread that set it, and only
            // while that thread is pumping messages. This loop is the delivery
            // mechanism rather than idle waiting.
            if !GetMessageW(&mut message, None, 0, 0).as_bool() {
                break;
            }
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }

        let _ = UnhookWindowsHookEx(hook);
    }
}

/// The hook itself. **Nothing slow may happen here.**
///
/// Windows gives a low-level hook `LowLevelHooksTimeout` milliseconds (300 by
/// default) to return before it removes it — without an error, and without any
/// way to notice other than the hook no longer firing. So this does the least
/// it can: a hit test, two class-name reads, and a non-blocking send.
///
/// All three calls are safe to make from here. `WindowFromPoint`, `GetAncestor`
/// and `GetClassNameW` read the window structures the kernel already holds;
/// none of them sends a message to the owning thread, so none of them can be
/// held up by a program that has stopped responding.
unsafe extern "system" fn on_mouse(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    // A negative code means this hook must pass the event on untouched.
    if code < 0 {
        return CallNextHookEx(None, code, wparam, lparam);
    }

    match wparam.0 as u32 {
        WM_RBUTTONDOWN => {
            let Some(mouse) = (lparam.0 as *const MSLLHOOKSTRUCT).as_ref() else {
                return CallNextHookEx(None, code, wparam, lparam);
            };
            if !is_desktop_at(mouse.pt) {
                return CallNextHookEx(None, code, wparam, lparam);
            }

            if let Some(clicks) = CLICKS.get() {
                // Unbounded, so this never blocks. A failure means the consumer
                // is gone, in which case the click should go to Explorer.
                if clicks.send((mouse.pt.x, mouse.pt.y)).is_err() {
                    return CallNextHookEx(None, code, wparam, lparam);
                }
            }

            ATE_THE_PRESS.store(true, Ordering::Relaxed);
            LRESULT(1)
        }
        // Only ever the release of a press this hook already ate — releases
        // that belong to Explorer must go through, or a drag ends nowhere.
        WM_RBUTTONUP if ATE_THE_PRESS.swap(false, Ordering::Relaxed) => LRESULT(1),
        _ => CallNextHookEx(None, code, wparam, lparam),
    }
}

/// Whether the window under this point is the desktop.
///
/// Explorer draws the desktop as a `SysListView32` (the icons) inside a
/// `SHELLDLL_DefView`, hosted by `Progman` or by one of the `WorkerW` windows.
/// Which of those is on top depends on the Windows version and on whether a
/// slideshow or a third-party wallpaper program has been near it, so all four
/// names are accepted rather than the one this machine happens to use today.
unsafe fn is_desktop_at(point: POINT) -> bool {
    let window = WindowFromPoint(point);
    if window.is_invalid() {
        return false;
    }

    if matches!(
        class_of(window).as_str(),
        "SysListView32" | "SHELLDLL_DefView"
    ) {
        return true;
    }

    let root = GetAncestor(window, GA_ROOT);
    if root.is_invalid() {
        return false;
    }
    // `Progman` and `WorkerW` alone are not enough — every WorkerW in the
    // session shares the class — but combined with the hit test they are: the
    // pointer is over one, and only the desktop's is ever hit.
    matches!(class_of(root).as_str(), "Progman" | "WorkerW")
}

unsafe fn class_of(window: HWND) -> String {
    // Class names are capped at 256 characters by the registration API, so a
    // buffer of that size cannot truncate one.
    let mut buffer = [0u16; 256];
    let written = GetClassNameW(window, &mut buffer);
    if written <= 0 {
        return String::new();
    }
    String::from_utf16_lossy(&buffer[..written as usize])
}
