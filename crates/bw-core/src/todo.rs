//! The to-do list behind the sidebar's To Do tab.
//!
//! The original keeps this in a plain JSON array of `{content, done}` objects
//! (`services/Todo.qml`), and reorders by index. Indices are a poor handle once
//! anything can be deleted concurrently — the sidebar sends "finish item 3"
//! while a rewrite has already shifted 3 to 4 — so this stores a stable id per
//! item and addresses everything by id. The on-disk shape stays an array, so
//! a file written by the original still loads.
//!
//! Like the notification store this lives in the portable crate: it is pure
//! data, so its tests run on Linux and its types reach the frontend through
//! ts-rs rather than being hand-copied into `ipc.ts`.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct TodoItem {
    /// Stable across reorders and reloads, unlike the array index the original
    /// addresses items by.
    pub id: u32,
    pub content: String,
    #[serde(default)]
    pub done: bool,
}

/// Items beyond this are refused rather than dropped.
///
/// A to-do list is authored by hand, so hitting this means something is wrong
/// — silently discarding the user's oldest tasks would be worse than saying no.
const MAX_ITEMS: usize = 500;

pub struct Store {
    inner: Mutex<Inner>,
    path: PathBuf,
}

struct Inner {
    items: Vec<TodoItem>,
    next_id: u32,
}

/// What a file on disk may hold: either this shell's shape (with ids) or the
/// original's (without). Reading both means an existing `todo.json` carries
/// over instead of appearing empty.
#[derive(Deserialize)]
struct StoredItem {
    #[serde(default)]
    id: Option<u32>,
    #[serde(default)]
    content: String,
    #[serde(default)]
    done: bool,
}

impl Store {
    /// Loads the list from `path`, starting empty if it cannot be read.
    pub fn load(path: PathBuf) -> Self {
        let stored: Vec<StoredItem> = std::fs::read_to_string(&path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default();

        // Items from the original have no id; number them as they are read.
        let mut next_id = stored.iter().filter_map(|item| item.id).max().unwrap_or(0);
        let items = stored
            .into_iter()
            .map(|item| TodoItem {
                id: item.id.unwrap_or_else(|| {
                    next_id = next_id.saturating_add(1);
                    next_id
                }),
                content: item.content,
                done: item.done,
            })
            .collect();

        Self {
            inner: Mutex::new(Inner {
                items,
                next_id: next_id.saturating_add(1),
            }),
            path,
        }
    }

    /// Recovers from a poisoned lock rather than panicking, as the notification
    /// store does: a to-do list is not worth taking the shell down for.
    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub fn list(&self) -> Vec<TodoItem> {
        self.lock().items.clone()
    }

    /// Appends a task. Returns `None` when the list is full or the text is
    /// blank — a blank task is invisible in the UI and impossible to delete.
    pub fn add(&self, content: impl Into<String>) -> Option<TodoItem> {
        let content = content.into().trim().to_owned();
        if content.is_empty() {
            return None;
        }

        let mut inner = self.lock();
        if inner.items.len() >= MAX_ITEMS {
            return None;
        }

        let item = TodoItem {
            id: inner.next_id,
            content,
            done: false,
        };
        inner.next_id = inner.next_id.saturating_add(1);
        inner.items.push(item.clone());

        self.persist_from(inner);
        Some(item)
    }

    /// Sets an item's done flag. Returns whether the item existed.
    pub fn set_done(&self, id: u32, done: bool) -> bool {
        let mut inner = self.lock();
        let Some(item) = inner.items.iter_mut().find(|item| item.id == id) else {
            return false;
        };
        item.done = done;
        self.persist_from(inner);
        true
    }

    pub fn remove(&self, id: u32) -> bool {
        let mut inner = self.lock();
        let before = inner.items.len();
        inner.items.retain(|item| item.id != id);
        if inner.items.len() == before {
            return false;
        }
        self.persist_from(inner);
        true
    }

    /// Drops every finished task, which is what the original's "clear" does.
    pub fn clear_done(&self) -> usize {
        let mut inner = self.lock();
        let before = inner.items.len();
        inner.items.retain(|item| !item.done);
        let removed = before - inner.items.len();
        if removed > 0 {
            self.persist_from(inner);
        }
        removed
    }

    /// Moves an item to `to`, shifting the rest along.
    ///
    /// Returns whether the move happened: an unknown id or an out-of-range
    /// target is a stale drag from a sidebar that has not caught up, not an
    /// error worth surfacing.
    pub fn reorder(&self, id: u32, to: usize) -> bool {
        let mut inner = self.lock();
        let Some(from) = inner.items.iter().position(|item| item.id == id) else {
            return false;
        };
        if to >= inner.items.len() || to == from {
            return false;
        }
        let item = inner.items.remove(from);
        inner.items.insert(to, item);
        self.persist_from(inner);
        true
    }

    /// Writes the list out, releasing the lock first so a slow disk cannot
    /// block a reader.
    fn persist_from(&self, inner: std::sync::MutexGuard<'_, Inner>) {
        let snapshot = inner.items.clone();
        drop(inner);

        if let Some(parent) = self.path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        // Failing to persist is not worth interrupting the user over: the list
        // is already correct in memory and already on screen.
        if let Ok(text) = serde_json::to_string_pretty(&snapshot) {
            let _ = std::fs::write(&self.path, text);
        }
    }
}

/// Where the list lives, alongside the other state the shell keeps.
pub fn todo_path() -> PathBuf {
    crate::paths::state_dir().join("todo.json")
}

/// Convenience for tests and for callers that only have a directory.
pub fn store_in(directory: &Path) -> Store {
    Store::load(directory.join("todo.json"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("bw-todo-{name}"));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("a writable temp dir");
        path
    }

    #[test]
    fn tasks_are_appended_in_the_order_they_are_added() {
        let store = store_in(&temp_dir("order"));
        store.add("First");
        store.add("Second");

        let list = store.list();
        assert_eq!(
            list.iter()
                .map(|item| item.content.as_str())
                .collect::<Vec<_>>(),
            ["First", "Second"]
        );
        assert_ne!(list[0].id, list[1].id);
    }

    #[test]
    fn blank_tasks_are_refused() {
        let store = store_in(&temp_dir("blank"));
        assert!(store.add("   ").is_none());
        assert!(store.add("").is_none());
        assert!(store.list().is_empty());
        // And leading whitespace is trimmed rather than stored.
        assert_eq!(store.add("  Real  ").unwrap().content, "Real");
    }

    #[test]
    fn done_toggles_and_clearing_keeps_the_unfinished() {
        let store = store_in(&temp_dir("done"));
        let first = store.add("Finish this").unwrap();
        store.add("But not this");

        assert!(store.set_done(first.id, true));
        assert!(store.list()[0].done);
        assert!(!store.set_done(9999, true), "an unknown id is not an error");

        assert_eq!(store.clear_done(), 1);
        let list = store.list();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].content, "But not this");
    }

    #[test]
    fn removing_takes_only_the_named_task() {
        let store = store_in(&temp_dir("remove"));
        let first = store.add("Go").unwrap();
        store.add("Stay");

        assert!(store.remove(first.id));
        assert!(!store.remove(first.id), "removing twice is not an error");
        assert_eq!(store.list().len(), 1);
        assert_eq!(store.list()[0].content, "Stay");
    }

    #[test]
    fn reordering_moves_one_task_and_shifts_the_rest() {
        let store = store_in(&temp_dir("reorder"));
        store.add("A");
        store.add("B");
        let third = store.add("C").unwrap();

        assert!(store.reorder(third.id, 0));
        assert_eq!(
            store
                .list()
                .iter()
                .map(|item| item.content.as_str())
                .collect::<Vec<_>>(),
            ["C", "A", "B"]
        );

        // A stale drag from a sidebar that has not caught up is ignored.
        assert!(!store.reorder(third.id, 99));
        assert!(!store.reorder(9999, 0));
    }

    #[test]
    fn the_list_survives_a_reload_without_reusing_ids() {
        let directory = temp_dir("reload");
        let highest = {
            let store = store_in(&directory);
            store.add("Kept");
            store.add("Also kept").unwrap().id
        };

        let reloaded = store_in(&directory);
        assert_eq!(reloaded.list().len(), 2);
        // Reusing an id would make finishing the new task tick off the old one.
        assert!(reloaded.add("New").unwrap().id > highest);
    }

    #[test]
    fn a_list_written_by_the_original_loads_and_gains_ids() {
        let directory = temp_dir("upstream");
        // The original stores no ids at all.
        std::fs::write(
            directory.join("todo.json"),
            r#"[{"content":"From upstream","done":true},{"content":"Second","done":false}]"#,
        )
        .unwrap();

        let store = store_in(&directory);
        let list = store.list();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].content, "From upstream");
        assert!(list[0].done);
        assert_ne!(list[0].id, list[1].id, "every item needs its own handle");

        // And the ids it hands out afterwards do not collide with those.
        let fresh = store.add("New").unwrap();
        assert!(list.iter().all(|item| item.id != fresh.id));
    }

    #[test]
    fn a_corrupt_list_starts_empty_rather_than_failing() {
        let directory = temp_dir("corrupt");
        std::fs::write(directory.join("todo.json"), "{ not json").unwrap();

        let store = store_in(&directory);
        assert!(store.list().is_empty());
        // And it recovers: adding still works and rewrites the file.
        store.add("Fine");
        assert_eq!(store_in(&directory).list().len(), 1);
    }

    #[test]
    fn the_list_is_bounded_by_refusing_rather_than_dropping() {
        let store = store_in(&temp_dir("bounded"));
        for index in 0..MAX_ITEMS {
            assert!(store.add(format!("#{index}")).is_some());
        }
        // The oldest tasks stay; the new one is refused.
        assert!(store.add("One too many").is_none());
        assert_eq!(store.list().len(), MAX_ITEMS);
        assert_eq!(store.list()[0].content, "#0");
    }
}
