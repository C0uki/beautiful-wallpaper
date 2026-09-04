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
                Ok(config) => adopt(&app, &state, config),
                // A half-written file is normal mid-save; the next event will
                // carry the complete one.
                Err(error) => tracing::debug!(%error, "ignoring an unreadable config"),
            }
        }
    });

    Some(watcher)
}

/// Takes a wholesale config change and re-does everything it invalidates.
///
/// The file watcher is one way a config arrives whole; applying a preset is
/// the other. Both need the same list of things to redo, and two copies of
/// that list would drift the first time something is added to it — silently,
/// because the symptom is a setting that takes effect when edited in the file
/// and not when a preset sets it.
///
/// The config is *not* written to disk here: the watcher's copy came from
/// disk, and the preset path saves before calling. Writing again would bounce
/// back through the watcher as another reload.
pub fn adopt(app: &AppHandle, state: &AppState, config: bw_core::Config) {
    let previous = state.config();

    let appearance_changed = config.appearance != previous.appearance;
    let keybinds_changed = config.keybinds != previous.keybinds;
    let menu_changed = config.hacks.desktop_menu != previous.hacks.desktop_menu
        || config.desktop_menu.enable != previous.desktop_menu.enable;
    let overlay_changed = config.overlay != previous.overlay;
    let chrome_changed = config.sidebar.corner_open != previous.sidebar.corner_open
        || config.appearance != previous.appearance
        || config.bar != previous.bar;
    let windows_changed = config.windows != previous.windows;
    let wallpaper = config.background.wallpaper_path.clone();
    let wallpaper_changed =
        wallpaper != previous.background.wallpaper_path && !wallpaper.is_empty();

    state.replace_config(config.clone());
    let _ = app.emit(event::CONFIG_CHANGED, &config);

    // Registrations are global to the process, so a changed chord only takes
    // effect if the old one is given back.
    if keybinds_changed {
        crate::services::hotkeys::apply(app);
    }

    // Switching the hack on or off in the file has to take effect without a
    // restart, like everything else here.
    if menu_changed {
        crate::services::deskmenu::apply(app);
    }

    // The hot corners' region is a property of their window, so a resized
    // strip needs re-cutting rather than just a repaint.
    if chrome_changed {
        crate::services::chrome::apply(app);
        crate::services::chrome::emit(app);
    }

    if overlay_changed {
        crate::services::overlay::apply(app);
    }

    // Hiding the taskbar and starting with Windows both reach out and change
    // the machine, so they follow the config like everything else here rather
    // than only being read once at startup.
    if windows_changed {
        crate::services::integration::apply(app);
    }

    // The wallpaper is the one setting that is not simply a value the surfaces
    // read: Windows has to be told, and the palette is generated from the
    // image. Without this, changing `background.wallpaperPath` in the file — or
    // through a preset — would move every surface to the new colours and leave
    // the desktop showing the old picture.
    //
    // This re-enters through the watcher once, because setting the wallpaper
    // writes the path back. The second pass sees the value it just wrote and
    // stops.
    if wallpaper_changed {
        match crate::services::wallpaper::apply(state, &wallpaper) {
            Ok(()) => {
                let _ = app.emit(
                    event::WALLPAPER_CHANGED,
                    serde_json::json!({ "monitor": "", "path": wallpaper, "blanked": false }),
                );
            }
            // A config carried over from another machine names a picture this
            // one may not have. Everything else in it still applied.
            Err(error) => {
                tracing::warn!(%error, %wallpaper, "that wallpaper is not on this machine")
            }
        }
    }

    if appearance_changed || wallpaper_changed {
        if let Ok(theme) = crate::services::theme::regenerate(state) {
            let _ = app.emit(event::THEME_CHANGED, &theme);
        }
    }
}
