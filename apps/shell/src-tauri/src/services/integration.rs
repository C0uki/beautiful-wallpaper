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

use tauri::{AppHandle, Manager};

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

/// Hides the taskbar again, if the shell is holding it hidden.
///
/// For a restarted Explorer, whose new taskbar can come up showing and set to
/// hold its edge again, having forgotten both.
pub fn reassert_taskbar(app: &AppHandle) {
    #[cfg(windows)]
    if let Some(held) = app.try_state::<Integration>() {
        if let Some(hidden) = held.taskbar.lock().as_ref() {
            unsafe { hidden.reassert() };
        }
    }
    #[cfg(not(windows))]
    let _ = app;
}

/// Has Explorer work every work area out again, when the shell is holding the
/// taskbar; see `HiddenTaskbar::recompute`. After the bars have asked for their
/// edges, so that it counts them.
pub fn recompute_work_areas(app: &AppHandle) {
    #[cfg(windows)]
    if let Some(held) = app.try_state::<Integration>() {
        if let Some(hidden) = held.taskbar.lock().as_ref() {
            unsafe { hidden.recompute() };
        }
    }
    #[cfg(not(windows))]
    let _ = app;
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
        (true, true) => {}
        // Not hidden, and not asked to be — but a shell killed while it was
        // may have left the taskbar set to hide itself. A no-op otherwise.
        (false, false) => unsafe { crate::platform::win::restore_saved_taskbar_state() },
        (true, false) => {
            let hidden = unsafe { crate::platform::win::HiddenTaskbar::hide() };
            let moved = hidden.moved();
            *guard = Some(hidden);
            drop(guard);
            // Moved to the bottom, it has given its edge back, and the bar
            // asks again for the space that leaves.
            if moved {
                crate::surfaces::desktop_changed(app);
            }
        }
        (false, true) => {
            let moved = guard.take().is_some_and(|hidden| hidden.moved());
            drop(guard);
            // Back on its own edge, it holds it again.
            if moved {
                crate::surfaces::desktop_changed(app);
            }
        }
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
    NotificationStore::announce(
        app,
        bw_core::NewNotification::from_shell("Starting with Windows", problem),
    );
}
