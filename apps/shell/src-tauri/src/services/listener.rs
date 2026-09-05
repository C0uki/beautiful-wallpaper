//! Following `hacks.readOtherNotifications`.
//!
//! Three things have to line up before this does anything, and the whole point
//! of the service is that it can say *which* of them is missing. A setting
//! that is on while nothing arrives is the failure this shell keeps trying not
//! to ship.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tauri::{AppHandle, Emitter, Manager};

use crate::commands::event;
use crate::state::{AppState, NotificationStore};

/// The reader's thread, and the switch that stops it.
#[derive(Default)]
pub struct ListenerHandle {
    running: Arc<AtomicBool>,
}

impl ListenerHandle {
    fn stop(&self) {
        self.running.store(false, Ordering::Relaxed);
    }
}

/// How often to look.
///
/// `UserNotificationListener` has a `NotificationChanged` event, but it is
/// only delivered to a packaged application with a background task registered
/// — which a sparse package over a Win32 executable is not. So this polls, and
/// the interval is a compromise: a toast the shell shows five seconds late is
/// still useful, and a second-by-second read of the whole Action Center is not
/// free.
#[cfg(windows)]
const INTERVAL: std::time::Duration = std::time::Duration::from_secs(4);

/// Starts or stops the reader so it matches the config.
pub fn apply(app: &AppHandle) {
    let wanted = app
        .state::<AppState>()
        .config()
        .hacks
        .read_other_notifications;

    let Some(handle) = app.try_state::<ListenerHandle>() else {
        return;
    };

    // Always stopped first: the thread reads the switch it was started with,
    // so restarting is how a changed setting takes effect at all.
    handle.stop();

    if !wanted {
        return;
    }

    #[cfg(windows)]
    start(app, handle.running.clone());
    #[cfg(not(windows))]
    let _ = app;
}

/// Removes the sparse package, when the setting has just been switched off.
///
/// Separate from [`apply`] on purpose: `apply` also runs at startup, and doing
/// this there would enumerate every package the user has installed on every
/// launch to remove one that is almost never there. Only a change to `false`
/// means anything.
pub fn forget() {
    #[cfg(windows)]
    if let Err(error) = crate::platform::identity::unregister() {
        // Worth saying in the log and not worth a notification: the setting is
        // off either way, and the package grants identity to an executable
        // that is still perfectly present.
        tracing::warn!(%error, "could not remove the sparse package");
    }
}

#[cfg(windows)]
fn start(app: &AppHandle, running: Arc<AtomicBool>) {
    use crate::platform::notifylisten::{self, Access};

    running.store(true, Ordering::Relaxed);
    let app = app.clone();

    std::thread::spawn(move || {
        // Identity first, because without it the listener will not talk to
        // this process at all — and registering the package does **not** give
        // identity to the process that registered it. Windows decides that
        // when a process starts, so this run of the shell can only set things
        // up and say so; the next one does the reading.
        if !crate::platform::identity::has_identity() {
            match crate::platform::identity::register() {
                Ok(()) => say(
                    &app,
                    "The notification package is registered. Restart the shell to finish \
                     switching this on — Windows decides a program's identity when it starts."
                        .to_owned(),
                ),
                Err(error) => say(&app, error),
            }
            running.store(false, Ordering::Relaxed);
            return;
        }

        // Asking is what puts the shell in Windows' notification-access list;
        // until then there is nothing there for the user to allow.
        match notifylisten::request() {
            Access::Allowed => {}
            other => {
                say(&app, describe(other));
                running.store(false, Ordering::Relaxed);
                return;
            }
        }

        // Fresh every time the reader starts: what is in the Action Center
        // while nobody is reading it is not an arrival either.
        let mut seen = bw_core::listener::Seen::new();

        while running.load(Ordering::Relaxed) {
            match notifylisten::read() {
                Ok(found) => {
                    let ids: Vec<u32> = found.iter().map(|(id, _)| *id).collect();
                    let arrivals = seen.arrivals(&ids);

                    if !arrivals.is_empty() {
                        post(&app, found, &arrivals);
                    }
                }
                // A transient failure is not worth a notification about
                // notifications; the next read will say the same thing or
                // succeed.
                Err(error) => tracing::debug!(%error, "could not read the Action Center"),
            }

            std::thread::sleep(INTERVAL);
        }
    });
}

#[cfg(windows)]
fn post(app: &AppHandle, found: Vec<(u32, bw_core::NewNotification)>, arrivals: &[u32]) {
    let Some(store) = app.try_state::<NotificationStore>() else {
        return;
    };

    for (id, notification) in found {
        if arrivals.contains(&id) {
            store.0.post(notification);
        }
    }

    let _ = app.emit(event::NOTIFICATIONS, store.0.list());
    let _ = crate::surfaces::set_visible(app, crate::surfaces::NOTIFICATIONS.label, true);
}

/// What to tell the user about a gate they are standing at.
///
/// Each of these is a different thing to do next, which is why the access
/// state is not a boolean.
#[cfg(windows)]
fn describe(access: crate::platform::notifylisten::Access) -> String {
    use crate::platform::notifylisten::Access;

    match access {
        // Reached only if identity was lost between the check above and the
        // request, which means the package was removed underneath the shell.
        Access::NoIdentity => {
            "The notification package is no longer registered. See docs/msix.md.".to_owned()
        }
        Access::Unavailable => "This Windows does not offer the notification listener.".to_owned(),
        Access::Denied => "Windows is set to refuse this shell access to notifications. Settings \
             › Privacy & security › Notifications is where that is undone."
            .to_owned(),
        Access::Unasked => "Windows has not been given an answer about notification access yet. \
             Settings › Privacy & security › Notifications is where it is given."
            .to_owned(),
        Access::Allowed => String::new(),
    }
}

#[cfg(windows)]
fn say(app: &AppHandle, problem: String) {
    let Some(store) = app.try_state::<NotificationStore>() else {
        return;
    };
    store.0.post(bw_core::NewNotification::from_shell(
        "Other applications' notifications",
        problem,
    ));
    let _ = app.emit(event::NOTIFICATIONS, store.0.list());
}
