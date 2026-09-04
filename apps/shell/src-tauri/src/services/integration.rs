//! The two settings that change Windows rather than the shell.
//!
//! `windows.hideSystemTaskbar` and `windows.startWithWindows` were both in the
//! schema, with a control each on the settings screen and one of them on the
//! first-run screen — and **nothing read either of them**. A switch that
//! writes a config key nothing acts on is the exact failure this project keeps
//! finding, and it is worse here than usual: both of these are the reason
//! somebody would open that screen at all.
//!
//! They are handled together because they are the same kind of thing. Neither
//! is a value a surface draws; each reaches out and changes the machine, and
//! each has to be undone.

use tauri::{AppHandle, Emitter, Manager};

use crate::commands::event;
use crate::state::{AppState, NotificationStore};

/// Holds whatever the shell has done to Windows on the machine's behalf.
///
/// The taskbar is a guard rather than a flag: hiding it changes the desktop
/// for every program on it, so putting it back is tied to a value's lifetime
/// rather than to a line of code remembering.
#[derive(Default)]
pub struct Integration {
    #[cfg(windows)]
    taskbar: parking_lot::Mutex<Option<crate::platform::win::HiddenTaskbar>>,
}

/// Makes Windows match what the config asks for.
///
/// Safe to call again: startup does, and the config watcher does whenever the
/// `windows` section changes, so a switch takes effect without a restart.
pub fn apply(app: &AppHandle) {
    let config = app.state::<AppState>().config();
    taskbar(app, config.windows.hide_system_taskbar);
    autostart(app, config.windows.start_with_windows);
}

/// Puts the taskbar back, whatever the config says.
///
/// Called when the shell is on its way out. The guard's own `Drop` does this
/// too, but only on the paths where anything is dropped.
pub fn restore(app: &AppHandle) {
    #[cfg(windows)]
    if let Some(held) = app.try_state::<Integration>() {
        drop(held.taskbar.lock().take());
    }
    #[cfg(not(windows))]
    let _ = app;
}

#[cfg(windows)]
fn taskbar(app: &AppHandle, hide: bool) {
    let Some(held) = app.try_state::<Integration>() else {
        return;
    };
    let mut guard = held.taskbar.lock();

    match (hide, guard.is_some()) {
        // Already in the state that was asked for. Not re-hiding matters: a
        // second `ShowWindow(SW_HIDE)` on an already-hidden taskbar is
        // harmless, but dropping and re-taking the guard would flash it.
        (true, true) | (false, false) => {}
        (true, false) => *guard = Some(unsafe { crate::platform::win::HiddenTaskbar::hide() }),
        (false, true) => drop(guard.take()),
    }
}

#[cfg(not(windows))]
fn taskbar(_app: &AppHandle, _hide: bool) {}

#[cfg(windows)]
fn autostart(app: &AppHandle, wanted: bool) {
    // Written every time rather than only when it changes: the path in the
    // entry is this executable's, and reinstalling somewhere else leaves the
    // old one behind as an error dialog at every login.
    if let Err(error) = crate::platform::autostart::apply(wanted) {
        tracing::warn!(%error, "could not change the auto-start entry");
        // Said rather than swallowed, for the same reason a refused hotkey is:
        // the switch is on, the machine disagrees, and nothing else would ever
        // tell the user which of the two is true.
        say(app, &error);
    }
}

#[cfg(not(windows))]
fn autostart(_app: &AppHandle, _wanted: bool) {}

#[cfg(windows)]
fn say(app: &AppHandle, problem: &str) {
    let Some(store) = app.try_state::<NotificationStore>() else {
        return;
    };
    store.0.post(bw_core::NewNotification::from_shell(
        "Starting with Windows",
        problem,
    ));
    let _ = app.emit(event::NOTIFICATIONS, store.0.list());
}
