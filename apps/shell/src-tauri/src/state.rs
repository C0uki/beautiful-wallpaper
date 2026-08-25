//! Shared shell state.
//!
//! One writer, many readers: the backend owns the config, the generated theme
//! and the surface flags, and every surface learns about changes through events.
//! That is the same discipline Quickshell's singletons enforce, transplanted to a
//! world where each surface is a separate process-level webview.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use bw_core::{config, Config, GeneratedTheme};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

/// The surface open/closed flags — end4-pC's `GlobalStates.qml`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct GlobalStates {
    pub wallpaper_selector_open: bool,
    pub sidebar_left_open: bool,
    pub sidebar_right_open: bool,
    pub overview_open: bool,
    pub settings_open: bool,
    pub session_open: bool,
    pub desktop_menu_open: bool,
    pub media_controls_open: bool,
    pub overlay_open: bool,
    pub widget_edit_mode: bool,
}

impl GlobalStates {
    /// Flips one flag by name, returning the new value.
    ///
    /// Going through JSON keeps this to one list of names — the struct's own —
    /// rather than a match arm that silently rots when a flag is added.
    pub fn toggle(&mut self, name: &str) -> Option<bool> {
        self.set(name, !self.get(name)?)
    }

    pub fn get(&self, name: &str) -> Option<bool> {
        let value = serde_json::to_value(self).ok()?;
        value.get(name)?.as_bool()
    }

    pub fn set(&mut self, name: &str, value: bool) -> Option<bool> {
        let mut json = serde_json::to_value(&*self).ok()?;
        let object = json.as_object_mut()?;
        if !object.contains_key(name) {
            return None;
        }
        object.insert(name.to_owned(), serde_json::Value::Bool(value));
        *self = serde_json::from_value(json).ok()?;
        Some(value)
    }
}

/// Which wallpaper each monitor is showing.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WallpaperState {
    pub monitor: String,
    pub path: String,
    pub blanked: bool,
}

pub struct Inner {
    pub config: RwLock<Config>,
    pub theme: RwLock<Option<GeneratedTheme>>,
    pub states: RwLock<GlobalStates>,
    pub wallpapers: RwLock<BTreeMap<String, WallpaperState>>,
    pub config_path: PathBuf,
    pub state_path: PathBuf,
}

/// The handle every command and provider holds.
#[derive(Clone)]
pub struct AppState(Arc<Inner>);

impl AppState {
    /// Loads the config from disk, materialising defaults on first run.
    pub fn load() -> Result<Self, config::ConfigError> {
        let config_path = bw_core::paths::config_file();
        let loaded = config::load(&config_path)?;

        // Writing the file back immediately means a first-run user has something
        // to edit, and that a config from an older version gains the new keys.
        config::save(&config_path, &loaded)?;

        Ok(Self(Arc::new(Inner {
            config: RwLock::new(loaded),
            theme: RwLock::new(None),
            states: RwLock::new(GlobalStates::default()),
            wallpapers: RwLock::new(BTreeMap::new()),
            config_path,
            state_path: bw_core::paths::state_file(),
        })))
    }

    pub fn config(&self) -> Config {
        self.0.config.read().clone()
    }

    pub fn theme(&self) -> Option<GeneratedTheme> {
        self.0.theme.read().clone()
    }

    pub fn set_theme(&self, theme: GeneratedTheme) {
        *self.0.theme.write() = Some(theme);
    }

    pub fn states(&self) -> GlobalStates {
        self.0.states.read().clone()
    }

    pub fn config_path(&self) -> &std::path::Path {
        &self.0.config_path
    }

    pub fn state_path(&self) -> &std::path::Path {
        &self.0.state_path
    }

    /// Replaces the config wholesale, as when the file changes on disk.
    pub fn replace_config(&self, config: Config) {
        *self.0.config.write() = config;
    }

    /// Applies a dotted-path edit and persists it.
    pub fn set_config_value(
        &self,
        path: &str,
        value: serde_json::Value,
    ) -> Result<Config, config::ConfigError> {
        let mut json = serde_json::to_value(self.config()).expect("config is serialisable");
        config::set_path(&mut json, path, value)?;

        let updated: Config =
            serde_json::from_value(json).map_err(|source| config::ConfigError::Parse {
                path: self.0.config_path.clone(),
                source,
            })?;

        config::save(&self.0.config_path, &updated)?;
        *self.0.config.write() = updated.clone();
        Ok(updated)
    }

    pub fn toggle_state(&self, name: &str) -> Option<GlobalStates> {
        let mut states = self.0.states.write();
        states.toggle(name)?;
        Some(states.clone())
    }

    pub fn set_state(&self, name: &str, value: bool) -> Option<GlobalStates> {
        let mut states = self.0.states.write();
        states.set(name, value)?;
        Some(states.clone())
    }

    pub fn record_wallpaper(&self, state: WallpaperState) {
        self.0
            .wallpapers
            .write()
            .insert(state.monitor.clone(), state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_toggle_by_name() {
        let mut states = GlobalStates::default();
        assert_eq!(states.toggle("wallpaperSelectorOpen"), Some(true));
        assert!(states.wallpaper_selector_open);
        assert_eq!(states.toggle("wallpaperSelectorOpen"), Some(false));
        assert!(!states.wallpaper_selector_open);
    }

    #[test]
    fn unknown_flags_are_rejected_rather_than_created() {
        let mut states = GlobalStates::default();
        assert_eq!(states.toggle("nopeOpen"), None);
        assert_eq!(states.set("nopeOpen", true), None);
        assert_eq!(states.get("nopeOpen"), None);
    }

    #[test]
    fn set_is_idempotent() {
        let mut states = GlobalStates::default();
        states.set("overviewOpen", true);
        states.set("overviewOpen", true);
        assert!(states.overview_open);
        assert_eq!(states.get("overviewOpen"), Some(true));
    }

    #[test]
    fn flag_names_are_camel_case_in_json() {
        let json = serde_json::to_value(GlobalStates::default()).unwrap();
        assert!(json.get("widgetEditMode").is_some());
        assert!(json.get("widget_edit_mode").is_none());
    }
}

/// The notification history, managed so commands can reach it.
pub struct NotificationStore(pub bw_core::notifications::Store);

/// The sidebar's to-do list.
pub struct TodoStore(pub bw_core::todo::Store);

impl Default for TodoStore {
    fn default() -> Self {
        Self(bw_core::todo::Store::load(bw_core::todo::todo_path()))
    }
}

/// Runtime state that is not configuration — which tab is open, whether the
/// bottom group is collapsed, how the toggle grid is arranged.
pub struct PersistentStore(pub bw_core::persistent::Store);

impl Default for PersistentStore {
    fn default() -> Self {
        Self(bw_core::persistent::Store::load(
            bw_core::paths::state_file(),
        ))
    }
}

impl Default for NotificationStore {
    fn default() -> Self {
        Self(bw_core::notifications::Store::load(
            bw_core::notifications::history_path(),
        ))
    }
}

/// The volume watcher, or nothing when audio is unavailable.
///
/// A machine with no output device is a normal state — a headless VM, or a
/// dock that has just been unplugged — so the shell keeps working and simply
/// reports silence rather than refusing to start.
#[derive(Default)]
pub struct VolumeHandle {
    #[cfg(windows)]
    watcher: Option<crate::platform::audio::VolumeWatcher>,
}

impl VolumeHandle {
    #[cfg(windows)]
    pub fn new(watcher: Option<crate::platform::audio::VolumeWatcher>) -> Self {
        Self { watcher }
    }

    #[cfg(not(windows))]
    pub fn new() -> Self {
        Self {}
    }

    #[cfg(windows)]
    pub fn read(&self) -> crate::providers::VolumeReading {
        self.watcher
            .as_ref()
            .map(|watcher| watcher.read())
            .unwrap_or_default()
    }

    #[cfg(not(windows))]
    pub fn read(&self) -> crate::providers::VolumeReading {
        crate::providers::VolumeReading::default()
    }

    pub fn set_percent(&self, _percent: f32, _ceiling: f32) -> Result<(), String> {
        #[cfg(windows)]
        if let Some(watcher) = self.watcher.as_ref() {
            return watcher
                .set_percent(_percent, _ceiling)
                .map_err(|error| error.to_string());
        }
        Ok(())
    }

    pub fn set_muted(&self, _muted: bool) -> Result<(), String> {
        #[cfg(windows)]
        if let Some(watcher) = self.watcher.as_ref() {
            return watcher.set_muted(_muted).map_err(|error| error.to_string());
        }
        Ok(())
    }
}

/// The brightness worker, or nothing when no display can report a level.
///
/// As with audio, "unavailable" is a normal state rather than an error: a
/// desktop whose monitor ignores DDC/CI has no brightness control, and the
/// shell hides the readout and the slider instead of showing a dead one.
#[derive(Default)]
pub struct BrightnessHandle {
    #[cfg(windows)]
    control: Option<crate::platform::brightness::BrightnessControl>,
}

impl BrightnessHandle {
    #[cfg(windows)]
    pub fn new(control: Option<crate::platform::brightness::BrightnessControl>) -> Self {
        Self { control }
    }

    #[cfg(not(windows))]
    pub fn new() -> Self {
        Self {}
    }

    /// The level now, or `None` when the platform cannot report one.
    pub fn read(&self) -> Option<u8> {
        #[cfg(windows)]
        return self.control.as_ref().and_then(|control| control.read());
        #[cfg(not(windows))]
        None
    }

    pub fn set(&self, _percent: u8) {
        #[cfg(windows)]
        if let Some(control) = self.control.as_ref() {
            control.set(_percent);
        }
    }

    /// Applies, or clears, the warm tint.
    pub fn set_night_light(&self, _kelvin: Option<u32>) {
        #[cfg(windows)]
        if let Some(control) = self.control.as_ref() {
            control.set_night_light(_kelvin);
        }
    }

    /// Re-reads the hardware, for when something outside the shell changed it.
    pub fn refresh(&self) {
        #[cfg(windows)]
        if let Some(control) = self.control.as_ref() {
            control.refresh();
        }
    }

    /// Whether to draw the control at all. A slider that moves but changes
    /// nothing is worse than no slider.
    pub fn is_supported(&self) -> bool {
        self.read().is_some()
    }
}

/// The microphone's level.
///
/// A newtype rather than a second copy of [`VolumeHandle`]: the two differ
/// only in which endpoint they follow, and Tauri distinguishes managed state
/// by type, so a wrapper is all that is needed to have both.
#[derive(Default)]
pub struct MicHandle(pub VolumeHandle);

/// The per-application volume mixer, or nothing when it could not be opened.
#[derive(Default)]
pub struct MixerHandle {
    #[cfg(windows)]
    mixer: Option<crate::platform::mixer::Mixer>,
}

impl MixerHandle {
    #[cfg(windows)]
    pub fn new(mixer: Option<crate::platform::mixer::Mixer>) -> Self {
        Self { mixer }
    }

    #[cfg(not(windows))]
    pub fn new() -> Self {
        Self {}
    }

    #[cfg(windows)]
    pub fn list(&self) -> Vec<crate::platform::mixer::SessionInfo> {
        self.mixer
            .as_ref()
            .map(|mixer| mixer.list())
            .unwrap_or_default()
    }

    #[cfg(not(windows))]
    pub fn list(&self) -> Vec<()> {
        Vec::new()
    }

    pub fn set_percent(&self, _id: &str, _percent: f32, _ceiling: f32) -> Result<(), String> {
        #[cfg(windows)]
        if let Some(mixer) = self.mixer.as_ref() {
            return mixer
                .set_percent(_id, _percent, _ceiling)
                .map_err(|error| error.to_string());
        }
        Ok(())
    }

    pub fn set_muted(&self, _id: &str, _muted: bool) -> Result<(), String> {
        #[cfg(windows)]
        if let Some(mixer) = self.mixer.as_ref() {
            return mixer
                .set_muted(_id, _muted)
                .map_err(|error| error.to_string());
        }
        Ok(())
    }
}

/// Whether the shell is holding the machine awake.
///
/// The Win32 request belongs to the thread that made it, so the inhibitor owns
/// a thread; this is only the handle onto it. On a host without that API the
/// flag is remembered but does nothing, which keeps the toggle honest in the
/// development harness rather than making it disappear.
#[derive(Default)]
pub struct IdleHandle {
    #[cfg(windows)]
    inhibitor: crate::platform::session::IdleInhibitor,
    #[cfg(not(windows))]
    on: std::sync::atomic::AtomicBool,
}

impl IdleHandle {
    pub fn is_on(&self) -> bool {
        #[cfg(windows)]
        return self.inhibitor.is_on();
        #[cfg(not(windows))]
        self.on.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn set(&self, on: bool) {
        #[cfg(windows)]
        self.inhibitor.set(on);
        #[cfg(not(windows))]
        self.on.store(on, std::sync::atomic::Ordering::Relaxed);
    }
}
