//! Watching `config.json` for outside edits.
//!
//! end4-pC uses a Quickshell `FileView { watchChanges: true }` with a 50 ms
//! debounce in both directions. The same contract holds here: edit the file in
//! any editor and the shell follows, without the shell's own writes bouncing
//! back as a reload.

use std::sync::mpsc;
use std::time::{Duration, Instant};

use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use tauri::{AppHandle, Emitter};

use crate::commands::event;
use crate::state::AppState;

/// Starts the watcher. The returned watcher must be kept alive for the life of
/// the app: dropping it silently stops the notifications.
pub fn watch(app: AppHandle, state: AppState) -> Option<RecommendedWatcher> {
    let path = state.config_path().to_path_buf();
    let directory = path.parent()?.to_path_buf();
    let debounce = Duration::from_millis(state.config().hacks.config_reload_delay.into());

    let (sender, receiver) = mpsc::channel();
    let mut watcher = notify::recommended_watcher(move |result| {
        let _ = sender.send(result);
    })
    .ok()?;
    watcher
        .watch(&directory, RecursiveMode::NonRecursive)
        .ok()?;

    std::thread::spawn(move || {
        let mut last = Instant::now() - debounce;
        for result in receiver {
            let Ok(notify_event) = result else { continue };
            if !notify_event.paths.iter().any(|changed| changed == &path) {
                continue;
            }
            // Editors save by writing several times in quick succession; without
            // this the shell would reload three times per save.
            if last.elapsed() < debounce {
                continue;
            }
            last = Instant::now();
            std::thread::sleep(debounce);

            match bw_core::config::load(&path) {
                Ok(config) => {
                    let appearance_changed = config.appearance != state.config().appearance;
                    let keybinds_changed = config.keybinds != state.config().keybinds;
                    state.replace_config(config.clone());
                    let _ = app.emit(event::CONFIG_CHANGED, &config);

                    // Registrations are global to the process, so a changed
                    // chord only takes effect if the old one is given back.
                    if keybinds_changed {
                        crate::services::hotkeys::apply(&app);
                    }

                    if appearance_changed {
                        if let Ok(theme) = crate::services::theme::regenerate(&state) {
                            let _ = app.emit(event::THEME_CHANGED, &theme);
                        }
                    }
                }
                // A half-written file is normal mid-save; the next event will
                // carry the complete one.
                Err(error) => tracing::debug!(%error, "ignoring an unreadable config"),
            }
        }
    });

    Some(watcher)
}
