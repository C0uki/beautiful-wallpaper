//! Where the shell keeps its files.
//!
//! end4-pC spreads state across `$XDG_CONFIG_HOME`, `$XDG_STATE_HOME` and
//! `$XDG_CACHE_HOME`. The Windows equivalents are Roaming (settings that belong
//! to the user), Local (machine-specific state) and a cache folder underneath it.

use std::path::PathBuf;

const APP_DIR: &str = "beautiful-wallpaper";

/// `%APPDATA%\beautiful-wallpaper` — user settings, worth roaming and backing up.
pub fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(APP_DIR)
}

/// `%LOCALAPPDATA%\beautiful-wallpaper` — generated themes, window geometry and
/// other state that is meaningless on another machine.
pub fn state_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(APP_DIR)
}

/// Thumbnails and downloaded previews. Safe to delete at any time.
pub fn cache_dir() -> PathBuf {
    state_dir().join("cache")
}

pub fn config_file() -> PathBuf {
    config_dir().join("config.json")
}

/// Runtime state kept separate from settings, as `Persistent.qml` does.
pub fn state_file() -> PathBuf {
    state_dir().join("state.json")
}

/// The generated Material scheme. Kept on disk, rather than only in memory, so
/// external tooling can read the current palette the way matugen's output could.
pub fn generated_theme_file() -> PathBuf {
    state_dir().join("generated").join("colors.json")
}

pub fn thumbnail_dir() -> PathBuf {
    cache_dir().join("thumbnails")
}

/// Where online wallpapers are downloaded when the user has not chosen a folder.
pub fn default_download_dir() -> PathBuf {
    dirs::picture_dir()
        .unwrap_or_else(|| dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")))
        .join("Wallpapers")
}

/// The folder the wallpaper browser opens at.
pub fn default_wallpaper_dir() -> PathBuf {
    dirs::picture_dir().unwrap_or_else(|| dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_path_sits_under_its_app_folder() {
        assert!(config_file().starts_with(config_dir()));
        assert!(state_file().starts_with(state_dir()));
        assert!(generated_theme_file().starts_with(state_dir()));
        assert!(thumbnail_dir().starts_with(cache_dir()));
        assert!(cache_dir().starts_with(state_dir()));
    }

    #[test]
    fn settings_and_state_do_not_share_a_directory() {
        // Roaming settings and local state must not collide, or backing up the
        // config would drag machine-specific state along with it.
        assert_ne!(config_dir(), state_dir());
    }

    #[test]
    fn the_app_folder_is_named_once() {
        assert!(config_dir().ends_with(APP_DIR));
        assert!(state_dir().ends_with(APP_DIR));
    }
}
