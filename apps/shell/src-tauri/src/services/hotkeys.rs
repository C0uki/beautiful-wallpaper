//! The keys that open the shell's surfaces.
//!
//! The original does not have this problem: Hyprland owns the keyboard, and a
//! keybind in its config runs `qs ipc call overview toggle`. Windows has no
//! such layer, so the shell registers system-wide hotkeys itself — and runs
//! into a constraint the original never had. **Windows reserves a large part
//! of the `Win`+letter space for its own shell** and refuses to hand those
//! combinations over, a lone `Win` press always opens the Start menu, and
//! which combinations are refused is neither documented nor stable across
//! versions and installed software.
//!
//! So a refusal is reported rather than swallowed. A key that silently does
//! nothing gives the user no way to find out why, and no reason to look in the
//! config file; a notification naming the binding gives them both.

use std::str::FromStr;

use bw_core::NewNotification;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

use crate::commands::event;
use crate::state::{AppState, NotificationStore};

/// Registers every configured key, replacing whatever was registered before.
///
/// Safe to call again: the config watcher does exactly that when `keybinds`
/// changes, so editing a chord takes effect without restarting the shell.
pub fn apply(app: &AppHandle) {
    let shortcuts = app.global_shortcut();
    // Registrations are global to the process, so the old set has to go before
    // the new one arrives or a removed binding would keep working.
    if let Err(error) = shortcuts.unregister_all() {
        tracing::warn!(%error, "could not clear the old keyboard shortcuts");
    }

    let config = app.state::<AppState>().config();
    if !config.keybinds.enable {
        return;
    }

    let mut refused: Vec<String> = Vec::new();

    for (binding, chord) in bindings(&config.keybinds) {
        let Some(bound) = action_for(&binding) else {
            // A binding for something this build does not have. Silence is
            // right here: the user did not ask for it.
            continue;
        };

        let Ok(shortcut) = Shortcut::from_str(&chord) else {
            refused.push(format!("{chord} ({binding})"));
            tracing::warn!(%chord, %binding, "not a key combination");
            continue;
        };

        let handle = app.clone();
        let registered = shortcuts.on_shortcut(shortcut, move |_app, _shortcut, pressed| {
            // Both the press and the release arrive; acting on both would
            // toggle the surface open and straight back closed.
            if pressed.state != ShortcutState::Pressed {
                return;
            }
            run(&handle, bound);
        });

        if let Err(error) = registered {
            refused.push(format!("{chord} ({binding})"));
            tracing::warn!(%error, %chord, %binding, "Windows would not give up this combination");
        }
    }

    report(app, &refused);
}

/// Each configured binding as a name and a chord, skipping the unassigned.
///
/// Read out of the serialised config rather than field by field, so adding a
/// key to the schema does not need a matching line here.
fn bindings(keybinds: &bw_core::config::Keybinds) -> Vec<(String, String)> {
    let Ok(serde_json::Value::Object(fields)) = serde_json::to_value(keybinds) else {
        return Vec::new();
    };

    fields
        .into_iter()
        .filter_map(|(name, value)| {
            let chord = value.as_str()?.trim().to_owned();
            (!chord.is_empty()).then_some((name, chord))
        })
        .collect()
}

/// What a key does when it is pressed.
#[derive(Debug, Clone, Copy)]
enum Bound {
    /// Flip a `GlobalStates` flag, which shows or hides a surface.
    Flag(&'static str),
    /// Take a picture of the screen. Not a toggle: pressing it twice should
    /// start two captures, not undo the first.
    Capture(bw_core::capture::CaptureMode),
}

/// What each binding is for.
///
/// Only things that exist are listed. A binding with nothing behind it would
/// register a key that does nothing, which is the failure this module is
/// written to avoid.
fn action_for(binding: &str) -> Option<Bound> {
    use bw_core::capture::CaptureMode;

    match binding {
        "overview" => Some(Bound::Flag("overviewOpen")),
        "sidebarLeft" => Some(Bound::Flag("sidebarLeftOpen")),
        "sidebarRight" => Some(Bound::Flag("sidebarRightOpen")),
        "wallpaperSelector" => Some(Bound::Flag("wallpaperSelectorOpen")),
        "widgetEditMode" => Some(Bound::Flag("widgetEditMode")),
        "captureRegion" => Some(Bound::Capture(CaptureMode::Screenshot)),
        "captureOcr" => Some(Bound::Capture(CaptureMode::Ocr)),
        "captureTranslate" => Some(Bound::Capture(CaptureMode::Translate)),
        _ => None,
    }
}

/// Runs what a key was bound to.
fn run(app: &AppHandle, bound: Bound) {
    match bound {
        Bound::Flag(flag) => toggle(app, flag),
        Bound::Capture(mode) => {
            let (Some(state), Some(capture)) = (
                app.try_state::<AppState>(),
                app.try_state::<crate::state::CaptureHandle>(),
            ) else {
                return;
            };
            if let Err(error) = crate::commands::start_capture(app.clone(), state, capture, mode) {
                tracing::warn!(%error, "could not start a capture");
            }
        }
    }
}

/// Flips a flag, exactly as the CLI and the surfaces do.
fn toggle(app: &AppHandle, flag: &str) {
    let Some(states) = app.state::<AppState>().toggle_state(flag) else {
        return;
    };
    crate::surfaces::apply_states(app, &states);
    let _ = app.emit(event::STATE_CHANGED, &states);
}

/// Says which keys could not be taken, once, rather than per key.
fn report(app: &AppHandle, refused: &[String]) {
    if refused.is_empty() {
        return;
    }
    let Some(store) = app.try_state::<NotificationStore>() else {
        return;
    };

    let notification = store.0.post(NewNotification::from_shell(
        "Some keyboard shortcuts are unavailable",
        format!(
            "Windows keeps these for itself, or another program already has them: {}. \
             Change them under `keybinds` in config.json.",
            refused.join(", ")
        ),
    ));
    let _ = app.emit(event::NOTIFICATIONS, store.0.list());
    let _ = notification;
    let _ = crate::surfaces::set_visible(app, crate::surfaces::NOTIFICATIONS.label, true);
}
