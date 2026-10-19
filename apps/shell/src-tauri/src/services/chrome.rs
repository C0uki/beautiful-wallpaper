//! Keeping the screen's decorations in step with the config.

use tauri::{AppHandle, Emitter, Manager};

use crate::commands::event;
use crate::state::AppState;

/// Tells every surface what the decorations should look like now.
pub fn emit(app: &AppHandle) {
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };

    let chrome = bw_core::chrome::ScreenChrome::resolve(&state.config());
    let _ = app.emit(event::CHROME, &chrome);
}
