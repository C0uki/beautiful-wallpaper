//! The shell's notification store.
//!
//! On Wayland the original *is* the notification server: applications hand it
//! their notifications over DBus. Windows has no such seat — reading other
//! applications' notifications needs `UserNotificationListener`, which needs
//! package identity, which needs the MSIX sparse package that is Phase 5 work.
//!
//! So the store is deliberately shaped around a single `post` entry point that
//! the shell itself feeds today and a listener can feed later, rather than
//! around the listener. Nothing above this module knows where a notification
//! came from.
//!
//! It lives in the portable crate rather than beside the Win32 code because it
//! contains no Windows at all — which means its tests actually run on Linux,
//! and its types reach the frontend through ts-rs instead of being copied by
//! hand into `ipc.ts`.

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// How loudly a notification asks to be seen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Urgency {
    Low,
    #[default]
    Normal,
    /// Stays up until dismissed, ignoring the configured timeout.
    Critical,
}

/// A button offered alongside a notification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct NotificationAction {
    pub id: String,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Notification {
    pub id: u32,
    /// Who it is from. Also what the toasts group by.
    pub app_name: String,
    pub summary: String,
    #[serde(default)]
    pub body: String,
    /// A path or URL for a thumbnail, empty when there is none.
    #[serde(default)]
    pub image: String,
    #[serde(default)]
    pub urgency: Urgency,
    /// Seconds since the epoch. Typed as a plain number on the TypeScript
    /// side: serde writes it as a JSON number, and it stays well inside the
    /// range a double represents exactly.
    #[ts(type = "number")]
    pub time: u64,
    #[serde(default)]
    pub actions: Vec<NotificationAction>,
}

/// What a caller supplies; the store assigns the id and the time.
#[derive(Debug, Clone, Default)]
pub struct NewNotification {
    pub app_name: String,
    pub summary: String,
    pub body: String,
    pub image: String,
    pub urgency: Urgency,
    pub actions: Vec<NotificationAction>,
}

impl NewNotification {
    /// The common case: the shell telling the user something it just did.
    pub fn from_shell(summary: impl Into<String>, body: impl Into<String>) -> Self {
        Self {
            app_name: "beautiful-wallpaper".to_owned(),
            summary: summary.into(),
            body: body.into(),
            ..Self::default()
        }
    }
}

/// Notifications beyond this are dropped oldest-first.
///
/// Unbounded history would grow forever in a process that is meant to run for
/// weeks, and nobody scrolls back past a few dozen.
const MAX_STORED: usize = 100;

/// The notification history, newest first.
pub struct Store {
    inner: Mutex<Inner>,
    path: PathBuf,
}

struct Inner {
    notifications: Vec<Notification>,
    next_id: u32,
}

impl Store {
    /// Loads the history from `path`, or starts empty if it cannot be read.
    ///
    /// A corrupt or missing file is not worth failing startup over — the shell
    /// still works, it just has no history.
    pub fn load(path: PathBuf) -> Self {
        let notifications = std::fs::read_to_string(&path)
            .ok()
            .and_then(|text| serde_json::from_str::<Vec<Notification>>(&text).ok())
            .unwrap_or_default();

        // Ids must not repeat after a restart, or dismissing one notification
        // would dismiss another.
        let next_id = notifications
            .iter()
            .map(|notification| notification.id)
            .max()
            .map_or(1, |highest| highest.saturating_add(1));

        Self {
            inner: Mutex::new(Inner {
                notifications,
                next_id,
            }),
            path,
        }
    }

    /// Recovers from a poisoned lock rather than panicking: a notification
    /// history is not worth taking the shell down for.
    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub fn list(&self) -> Vec<Notification> {
        self.lock().notifications.clone()
    }

    /// Records a notification and returns it, id and time filled in.
    pub fn post(&self, new: NewNotification) -> Notification {
        let mut inner = self.lock();

        let notification = Notification {
            id: inner.next_id,
            app_name: new.app_name,
            summary: new.summary,
            body: new.body,
            image: new.image,
            urgency: new.urgency,
            time: now_seconds(),
            actions: new.actions,
        };
        inner.next_id = inner.next_id.saturating_add(1);

        inner.notifications.insert(0, notification.clone());
        inner.notifications.truncate(MAX_STORED);

        let snapshot = inner.notifications.clone();
        drop(inner);
        self.persist(&snapshot);

        notification
    }

    /// Removes one notification. Returns whether it was there.
    pub fn dismiss(&self, id: u32) -> bool {
        let mut inner = self.lock();
        let before = inner.notifications.len();
        inner
            .notifications
            .retain(|notification| notification.id != id);
        let removed = inner.notifications.len() != before;

        if removed {
            let snapshot = inner.notifications.clone();
            drop(inner);
            self.persist(&snapshot);
        }
        removed
    }

    pub fn clear(&self) {
        let mut inner = self.lock();
        inner.notifications.clear();
        drop(inner);
        self.persist(&[]);
    }

    fn persist(&self, notifications: &[Notification]) {
        if let Some(parent) = self.path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        // Failing to persist is not worth interrupting the user over: the
        // notification is already in memory and already on screen.
        if let Ok(text) = serde_json::to_string_pretty(notifications) {
            let _ = std::fs::write(&self.path, text);
        }
    }
}

/// Where the history lives, alongside the other state the shell keeps.
pub fn history_path() -> PathBuf {
    crate::paths::state_dir().join("notifications.json")
}

fn now_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|since| since.as_secs())
        .unwrap_or_default()
}

/// Convenience for tests and for callers that only have a directory.
pub fn store_in(directory: &Path) -> Store {
    Store::load(directory.join("notifications.json"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("bw-notifications-{name}"));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("a writable temp dir");
        path
    }

    #[test]
    fn posting_assigns_ids_and_puts_the_newest_first() {
        let store = store_in(&temp_dir("order"));

        let first = store.post(NewNotification::from_shell("First", ""));
        let second = store.post(NewNotification::from_shell("Second", ""));

        assert_ne!(first.id, second.id);
        let list = store.list();
        assert_eq!(list[0].summary, "Second");
        assert_eq!(list[1].summary, "First");
    }

    #[test]
    fn dismissing_removes_only_the_named_one() {
        let store = store_in(&temp_dir("dismiss"));
        let first = store.post(NewNotification::from_shell("First", ""));
        store.post(NewNotification::from_shell("Second", ""));

        assert!(store.dismiss(first.id));
        assert!(!store.dismiss(first.id), "dismissing twice is not an error");

        let list = store.list();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].summary, "Second");
    }

    #[test]
    fn the_history_survives_a_reload_without_reusing_ids() {
        let directory = temp_dir("reload");
        let highest = {
            let store = store_in(&directory);
            store.post(NewNotification::from_shell("Kept", "across a restart"));
            store.post(NewNotification::from_shell("Also kept", "")).id
        };

        let reloaded = store_in(&directory);
        assert_eq!(reloaded.list().len(), 2);

        // Reusing an id would make dismissing the new one remove the old.
        let fresh = reloaded.post(NewNotification::from_shell("New", ""));
        assert!(fresh.id > highest, "{} should exceed {highest}", fresh.id);
    }

    #[test]
    fn the_history_is_bounded() {
        let store = store_in(&temp_dir("bounded"));
        for index in 0..MAX_STORED + 20 {
            store.post(NewNotification::from_shell(format!("#{index}"), ""));
        }

        let list = store.list();
        assert_eq!(list.len(), MAX_STORED);
        // The oldest are the ones that go.
        assert_eq!(list[0].summary, format!("#{}", MAX_STORED + 19));
    }

    #[test]
    fn a_corrupt_history_starts_empty_rather_than_failing() {
        let directory = temp_dir("corrupt");
        std::fs::write(directory.join("notifications.json"), "{ not json").unwrap();

        let store = store_in(&directory);
        assert!(store.list().is_empty());
        // And it recovers: posting still works and rewrites the file.
        store.post(NewNotification::from_shell("Fine", ""));
        assert_eq!(store.list().len(), 1);
    }

    #[test]
    fn urgency_serialises_in_the_shape_the_frontend_reads() {
        let json = serde_json::to_string(&Urgency::Critical).unwrap();
        assert_eq!(json, "\"critical\"");
        assert_eq!(Urgency::default(), Urgency::Normal);
    }
}
