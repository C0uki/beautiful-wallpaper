//! Keeping the overlay's two windows in step with what is pinned.
//!
//! The overlay is unusual among the surfaces in that nothing else can own its
//! visibility. An ordinary overlay is shown by its `GlobalStates` flag and
//! hidden when the flag clears; this one has to stay on screen after the flag
//! clears if anything on it was pinned, and it has to be cut down to just
//! those widgets when it does — a full-screen window that is neither hidden
//! nor masked swallows every click on the desktop.
//!
//! So `apply` is the single place that decides, and every path that could
//! change the answer calls it: the flag, a drag, a pin, a config reload.

use tauri::{AppHandle, Emitter, Manager};

use crate::commands::event;
use crate::state::{AppState, PersistentStore};

/// Re-reads the layout, moves both windows to match it, and says so.
pub fn apply(app: &AppHandle) {
    let (Some(state), Some(persistent)) = (
        app.try_state::<AppState>(),
        app.try_state::<PersistentStore>(),
    ) else {
        return;
    };

    let config = state.config();
    let open = state.states().overlay_open;
    let layout = bw_core::overlay::layout(
        &persistent.0.get().overlay,
        &config.overlay,
        screen_size(app),
        open,
    );

    show(
        app,
        crate::surfaces::OVERLAY.label,
        layout.interactive_visible,
        open,
    );
    show(
        app,
        crate::surfaces::OVERLAY_PINNED.label,
        layout.passive_visible,
        false,
    );

    #[cfg(windows)]
    apply_region(app, &layout);

    let _ = app.emit(event::OVERLAY, &layout);
}

/// Shows or hides one of the two windows, taking the focus only when asked.
///
/// The open overlay wants the keyboard — Escape closes it and a note is there
/// to be typed into. The same window left behind by a pinned widget must not
/// take it: the user is playing a game, and a shell that stole the keyboard
/// when a crosshair appeared would be worse than no crosshair.
fn show(app: &AppHandle, label: &str, visible: bool, focus: bool) {
    let Some(window) = app.get_webview_window(label) else {
        return;
    };
    if !visible {
        let _ = window.hide();
        return;
    }
    let _ = window.show();
    if focus {
        let _ = window.set_focus();
    }
}

/// Cuts the interactive window down to whatever is still live on it.
#[cfg(windows)]
fn apply_region(app: &AppHandle, layout: &bw_core::overlay::OverlayLayout) {
    use windows::Win32::Foundation::HWND;

    let Some(window) = app.get_webview_window(crate::surfaces::OVERLAY.label) else {
        return;
    };
    let Ok(handle) = window.hwnd() else {
        return;
    };
    let hwnd = HWND(handle.0);

    match &layout.region {
        // Open: the whole window is live, including the backdrop, because
        // clicking past the widgets is how the overlay is dismissed.
        None => {
            if let Err(error) = crate::platform::region::set_window_region(hwnd, &[]) {
                tracing::warn!(%error, "could not give the overlay its whole window back");
            }
        }
        Some(rects) if rects.is_empty() => {
            // Nothing live. The window is already hidden by `show`; clearing
            // the region here would hand it back whole, and if anything ever
            // showed it again it would cover the desktop.
        }
        Some(rects) => {
            if let Err(error) = crate::platform::region::set_window_region(hwnd, rects) {
                tracing::warn!(%error, "could not shape the overlay to its pinned widgets");
                // Better no pinned widgets than a full-screen window that is
                // neither hidden nor masked.
                let _ = window.hide();
            }
        }
    }
}

/// Makes the passive window ignore the pointer, once, after it is created.
///
/// It only ever draws things that are explicitly see-through, so unlike its
/// sibling it is masked by transparency rather than by a region.
pub fn make_passive_clickthrough(_app: &AppHandle) {
    #[cfg(windows)]
    {
        let Some(window) = _app.get_webview_window(crate::surfaces::OVERLAY_PINNED.label) else {
            return;
        };
        let Ok(handle) = window.hwnd() else {
            return;
        };
        unsafe {
            crate::platform::win::set_click_through(
                windows::Win32::Foundation::HWND(handle.0),
                true,
            );
        }
    }
}

fn screen_size(app: &AppHandle) -> (i32, i32) {
    app.primary_monitor()
        .ok()
        .flatten()
        .map(|monitor| {
            let size = monitor.size();
            (size.width as i32, size.height as i32)
        })
        .unwrap_or((1920, 1080))
}
