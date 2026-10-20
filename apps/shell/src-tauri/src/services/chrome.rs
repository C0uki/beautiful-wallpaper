//! Keeping the screen's decorations in step with the config.

use tauri::{AppHandle, Emitter, Manager};

use crate::commands::event;
use crate::state::AppState;
use crate::surfaces::{self, SCREEN_CHROME};

/// Tells every surface what the decorations should look like now, and makes
/// the window that draws the frame, or takes it down: a whole webview, tens of
/// megabytes, kept for a frame that is off by default was waste.
pub fn emit(app: &AppHandle) {
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };

    let chrome = bw_core::chrome::ScreenChrome::resolve(&state.config());
    let framed = !chrome.frame_edges.is_empty();
    // On the main thread, where windows are made, and outside whatever asked.
    let handle = app.clone();
    surfaces::later(app, move || {
        match handle.get_webview_window(SCREEN_CHROME.label) {
            Some(window) if !framed => {
                let _ = window.destroy();
            }
            None if framed => {
                if let Err(error) = surfaces::ensure(&handle, &SCREEN_CHROME) {
                    tracing::error!(%error, "could not make the frame");
                }
            }
            _ => {}
        }
    });
    let _ = app.emit(event::CHROME, &chrome);
}
