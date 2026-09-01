//! Keeping the screen's decorations in step with the config and the desktop.
//!
//! Two things are applied here rather than in the surfaces. The hot corners'
//! window region is a Win32 property of the window, not something a page can
//! set; and whether that window should exist at all depends on the config,
//! which can change while the shell is running.

use tauri::{AppHandle, Emitter, Manager};

use crate::commands::event;
use crate::state::{AppState, ChromeState};

/// Re-applies the hot corners' region, and shows or hides their window.
///
/// Safe to call again: the config watcher does exactly that, so editing the
/// corner size takes effect without restarting the shell.
pub fn apply(app: &AppHandle) {
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    let config = state.config();
    let fullscreen = app
        .try_state::<ChromeState>()
        .map(|chrome| chrome.is_fullscreen())
        .unwrap_or(false);

    let corners = bw_core::chrome::hot_corners(&config.sidebar.corner_open, screen_size(app));
    let wanted = !corners.is_empty() && !fullscreen;

    // Hidden rather than given an empty region: clearing a window's region
    // hands the whole window back, and a full-screen window that is not
    // click-through would swallow every click on the desktop.
    if let Err(error) =
        crate::surfaces::set_visible(app, crate::surfaces::HOT_CORNERS.label, wanted)
    {
        tracing::warn!(%error, "could not change the hot corners' visibility");
    }
    if !wanted {
        return;
    }

    #[cfg(windows)]
    {
        let Some(window) = app.get_webview_window(crate::surfaces::HOT_CORNERS.label) else {
            return;
        };
        let Ok(handle) = window.hwnd() else {
            tracing::warn!("the hot corners have no window handle yet");
            return;
        };
        let hwnd = windows::Win32::Foundation::HWND(handle.0);

        let rects: Vec<bw_core::capture::Rect> = corners.iter().map(|corner| corner.rect).collect();
        if let Err(error) = crate::platform::region::set_window_region(hwnd, &rects) {
            tracing::warn!(%error, "could not shape the hot corners");
            // Better no hot corners than a full-screen window that eats every
            // click because its region was never applied.
            let _ = crate::surfaces::set_visible(app, crate::surfaces::HOT_CORNERS.label, false);
            return;
        }

        // The region is the mask, so this window must *not* be click-through:
        // everything outside the strips already falls through it.
        unsafe {
            crate::platform::win::set_click_through(hwnd, false);
        }
    }
}

/// Tells every surface what the decorations should look like now.
pub fn emit(app: &AppHandle) {
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    let fullscreen = app
        .try_state::<ChromeState>()
        .map(|chrome| chrome.is_fullscreen())
        .unwrap_or(false);

    let chrome = bw_core::chrome::ScreenChrome::resolve(&state.config(), fullscreen);
    let _ = app.emit(event::CHROME, &chrome);
}

/// The primary monitor in physical pixels, which is what a region is measured in.
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
