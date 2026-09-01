//! Runtime state that is not configuration.
//!
//! The original draws this line too (`modules/common/Persistent.qml` versus
//! `Config.qml`): which sidebar tab was open, whether a group is collapsed,
//! whether sleep is being inhibited. None of it is something a user would sit
//! down and edit, and none of it should end up in a config file people share
//! or copy between machines — so it lives under `paths::state_file()` while
//! `config.json` stays hand-editable.
//!
//! Unlike the config schema this deliberately does *not* deny unknown fields.
//! A state file written by a newer build must still load in an older one; the
//! worst acceptable outcome is a forgotten tab, never a shell that refuses to
//! start.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

fn s(value: &str) -> String {
    value.to_owned()
}

/// Declares a state struct with camelCase keys, defaults for every field, and
/// TypeScript bindings. Deliberately tolerant of unknown fields.
macro_rules! state_struct {
    (
        $(#[$meta:meta])*
        pub struct $name:ident {
            $(
                $(#[doc = $doc:literal])*
                pub $field:ident : $ty:ty = $default:expr,
            )*
        }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
        #[serde(rename_all = "camelCase", default)]
        #[ts(export)]
        pub struct $name {
            $(
                $(#[doc = $doc])*
                pub $field: $ty,
            )*
        }

        impl Default for $name {
            fn default() -> Self {
                Self {
                    $( $field: $default, )*
                }
            }
        }
    };
}

state_struct! {
    /// Root of `state.json`.
    pub struct Persistent {
        pub sidebar: SidebarState = SidebarState::default(),
        pub idle: IdleState = IdleState::default(),
        pub overlay: OverlayState = OverlayState::default(),
    }
}

state_struct! {
    pub struct SidebarState {
        pub bottom_group: BottomGroupState = BottomGroupState::default(),
        /// The Android-style toggle grid's layout, in display order. Empty
        /// means "the built-in order", so a fresh install and a user who has
        /// never opened the editor both get the default arrangement.
        pub quick_toggles: Vec<ToggleSlot> = Vec::new(),
    }
}

state_struct! {
    pub struct BottomGroupState {
        /// Index into the tab list — calendar, to-do, timer.
        pub tab: u32 = 0,
        pub collapsed: bool = false,
    }
}

state_struct! {
    /// Which overlay widgets are out, and where the user left them.
    pub struct OverlayState {
        /// The widgets currently placed on the canvas, by keyword.
        pub open: Vec<String> = vec![s("crosshair"), s("resources")],
        pub crosshair: OverlayWidgetState = OverlayWidgetState::centred_clickthrough(),
        pub notes: OverlayWidgetState = OverlayWidgetState::at(80, 120),
        pub resources: OverlayWidgetState = OverlayWidgetState::at(80, 380),
    }
}

state_struct! {
    /// One widget's place on the canvas.
    pub struct OverlayWidgetState {
        /// Stay on screen after the overlay closes.
        pub pinned: bool = false,
        /// Let the pointer through, so what is underneath still works.
        pub clickthrough: bool = false,
        /// Physical screen pixels, as the window region measures them.
        pub x: i32 = 80,
        pub y: i32 = 80,
        /// Zero means "whatever this widget's own default is".
        pub width: i32 = 0,
        pub height: i32 = 0,
    }
}

impl OverlayWidgetState {
    fn at(x: i32, y: i32) -> Self {
        Self {
            x,
            y,
            ..Self::default()
        }
    }

    /// The crosshair's starting state: on the canvas and see-through, but
    /// **not** pinned. Click-through is the only mode it is useful in, so that
    /// is the default; pinning it is a decision, because a pinned crosshair is
    /// on screen over everything until somebody takes it away, and nobody
    /// asked for one by installing a shell.
    fn centred_clickthrough() -> Self {
        Self {
            clickthrough: true,
            x: 928,
            y: 508,
            ..Self::default()
        }
    }
}

state_struct! {
    pub struct IdleState {
        /// Whether the shell is holding the display awake.
        pub inhibit: bool = false,
    }
}

state_struct! {
    /// One tile in the Android-style quick-toggle grid.
    pub struct ToggleSlot {
        /// Matches a toggle's id in the frontend's registry.
        pub id: String = String::new(),
        pub enabled: bool = true,
        /// Whether the tile spans both columns.
        pub wide: bool = false,
    }
}

/// The state file, loaded once and written back on every change.
pub struct Store {
    inner: Mutex<Persistent>,
    path: PathBuf,
}

impl Store {
    /// Loads the state from `path`, falling back to defaults if it cannot be
    /// read or parsed.
    pub fn load(path: PathBuf) -> Self {
        let state = std::fs::read_to_string(&path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default();

        Self {
            inner: Mutex::new(state),
            path,
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Persistent> {
        self.inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub fn get(&self) -> Persistent {
        self.lock().clone()
    }

    /// Applies a dotted-path edit, the same vocabulary `config.set` uses, so
    /// the frontend has one way to write both files.
    pub fn set_path(&self, path: &str, value: serde_json::Value) -> Result<Persistent, StateError> {
        let mut json = serde_json::to_value(self.get()).expect("state is serialisable");
        set_in(&mut json, path, value)?;

        let updated: Persistent =
            serde_json::from_value(json).map_err(|source| StateError::Parse(source.to_string()))?;

        *self.lock() = updated.clone();
        self.persist(&updated);
        Ok(updated)
    }

    /// Replaces the state wholesale.
    pub fn replace(&self, state: Persistent) {
        *self.lock() = state.clone();
        self.persist(&state);
    }

    fn persist(&self, state: &Persistent) {
        if let Some(parent) = self.path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        // As with the other stores: the value is already correct in memory, so
        // a failed write is not worth interrupting the user over.
        if let Ok(text) = serde_json::to_string_pretty(state) {
            let _ = std::fs::write(&self.path, text);
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum StateError {
    #[error("no such state key: {0}")]
    UnknownKey(String),
    #[error("state would no longer parse: {0}")]
    Parse(String),
}

/// Walks a dotted path, creating nothing: an unknown key is a caller mistake,
/// not an invitation to invent a field that nothing reads.
fn set_in(
    root: &mut serde_json::Value,
    path: &str,
    value: serde_json::Value,
) -> Result<(), StateError> {
    let mut current = root;
    let mut walked = Vec::new();

    let mut segments = path.split('.').peekable();
    while let Some(segment) = segments.next() {
        walked.push(segment);
        let object = current
            .as_object_mut()
            .ok_or_else(|| StateError::UnknownKey(walked.join(".")))?;
        if !object.contains_key(segment) {
            return Err(StateError::UnknownKey(walked.join(".")));
        }

        if segments.peek().is_none() {
            object.insert(segment.to_owned(), value);
            return Ok(());
        }
        current = object.get_mut(segment).expect("checked just above");
    }

    Err(StateError::UnknownKey(path.to_owned()))
}

/// Convenience for tests and for callers that only have a directory.
pub fn store_in(directory: &Path) -> Store {
    Store::load(directory.join("state.json"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn temp_dir(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("bw-state-{name}"));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("a writable temp dir");
        path
    }

    #[test]
    fn defaults_open_the_first_tab_uncollapsed() {
        let state = Persistent::default();
        assert_eq!(state.sidebar.bottom_group.tab, 0);
        assert!(!state.sidebar.bottom_group.collapsed);
        assert!(!state.idle.inhibit);
        assert!(state.sidebar.quick_toggles.is_empty());
    }

    #[test]
    fn keys_are_camel_case_on_disk() {
        let json = serde_json::to_value(Persistent::default()).unwrap();
        assert!(json["sidebar"]["bottomGroup"].is_object());
        assert!(json["sidebar"].get("bottom_group").is_none());
        assert!(json["sidebar"]["quickToggles"].is_array());
    }

    #[test]
    fn a_dotted_edit_reaches_a_nested_field_and_persists() {
        let directory = temp_dir("edit");
        let store = store_in(&directory);

        let updated = store
            .set_path("sidebar.bottomGroup.tab", json!(2))
            .expect("a known key");
        assert_eq!(updated.sidebar.bottom_group.tab, 2);

        // And it is on disk, not only in memory.
        assert_eq!(store_in(&directory).get().sidebar.bottom_group.tab, 2);
    }

    #[test]
    fn an_unknown_key_is_refused_rather_than_invented() {
        let store = store_in(&temp_dir("unknown"));
        assert!(matches!(
            store.set_path("sidebar.nope", json!(1)),
            Err(StateError::UnknownKey(_))
        ));
        assert!(matches!(
            store.set_path("nope.at.all", json!(1)),
            Err(StateError::UnknownKey(_))
        ));
    }

    #[test]
    fn an_edit_of_the_wrong_type_leaves_the_state_alone() {
        let store = store_in(&temp_dir("wrong-type"));
        assert!(store.set_path("idle.inhibit", json!("yes")).is_err());
        // The in-memory state must not have taken the bad value on the way out.
        assert!(!store.get().idle.inhibit);
    }

    #[test]
    fn state_from_a_newer_build_loads_instead_of_failing() {
        let directory = temp_dir("forward");
        // A key this build has never heard of, alongside one it has.
        std::fs::write(
            directory.join("state.json"),
            r#"{"sidebar":{"bottomGroup":{"tab":1}},"somethingNewer":{"a":1}}"#,
        )
        .unwrap();

        let state = store_in(&directory).get();
        assert_eq!(state.sidebar.bottom_group.tab, 1);
        // Fields the newer file omitted still come back as defaults.
        assert!(!state.sidebar.bottom_group.collapsed);
    }

    #[test]
    fn a_corrupt_state_file_falls_back_to_defaults() {
        let directory = temp_dir("corrupt");
        std::fs::write(directory.join("state.json"), "{ not json").unwrap();

        let store = store_in(&directory);
        assert_eq!(store.get(), Persistent::default());
        // And it recovers: an edit still works and rewrites the file.
        store.set_path("idle.inhibit", json!(true)).unwrap();
        assert!(store_in(&directory).get().idle.inhibit);
    }

    #[test]
    fn the_toggle_layout_round_trips() {
        let directory = temp_dir("toggles");
        let store = store_in(&directory);
        store
            .set_path(
                "sidebar.quickToggles",
                json!([{"id": "wifi", "enabled": true, "wide": true}]),
            )
            .unwrap();

        let slots = store_in(&directory).get().sidebar.quick_toggles;
        assert_eq!(slots.len(), 1);
        assert_eq!(slots[0].id, "wifi");
        assert!(slots[0].wide);
    }
}
