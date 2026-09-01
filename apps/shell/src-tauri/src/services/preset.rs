//! Saving the config under a name, and putting one back.
//!
//! The rules — what a name may be, what a preset would change, and which of
//! those changes are safe to write — are in `bw_core::preset` under tests.
//! What is here is the part that needs a running shell: the folder, the undo,
//! and everything that has to be redone once a couple of hundred settings
//! change at once.

use std::path::PathBuf;

use bw_core::preset::{self, Comparison, PresetError, PresetSummary};
use serde_json::Value;
use tauri::{AppHandle, Manager};

use crate::state::{AppState, PresetUndo};

fn folder() -> PathBuf {
    bw_core::paths::presets_dir()
}

pub fn list() -> Vec<PresetSummary> {
    preset::list(&folder())
}

/// Saves the live config under a name.
pub fn save(
    state: &AppState,
    name: &str,
    description: &str,
    overwrite: bool,
) -> Result<Vec<PresetSummary>, PresetError> {
    let config = serde_json::to_value(state.config()).expect("the config is serialisable");
    preset::save(&folder(), name, description, &config, overwrite)?;
    Ok(list())
}

pub fn remove(name: &str) -> Result<Vec<PresetSummary>, PresetError> {
    preset::remove(&folder(), name)?;
    Ok(list())
}

/// What applying this preset would change, for the list shown before Apply.
pub fn compare(state: &AppState, name: &str) -> Result<Comparison, PresetError> {
    let stored = preset::load(&folder(), name)?;
    let current = serde_json::to_value(state.config()).expect("the config is serialisable");
    Ok(preset::compare(&current, &stored.config))
}

/// Applies the named changes from a preset.
///
/// `paths` is what [`compare`] offered, minus anything the user unticked.
pub fn apply(
    app: &AppHandle,
    state: &AppState,
    name: &str,
    paths: &[String],
) -> Result<bw_core::Config, PresetError> {
    let stored = preset::load(&folder(), name)?;
    let previous = state.config();

    let mut json = serde_json::to_value(&previous).expect("the config is serialisable");
    preset::apply(&mut json, &stored.config, paths)?;
    let updated = to_config(json, state)?;

    // Remembered before anything is written, and only here: applying a preset
    // changes a couple of hundred settings in one press, and without this the
    // way back is to remember what every one of them used to be.
    app.state::<PresetUndo>().hold(previous);

    adopt(app, state, updated.clone());
    Ok(updated)
}

/// Puts back the config the last preset replaced.
pub fn undo(app: &AppHandle, state: &AppState) -> Result<bw_core::Config, String> {
    let handle = app.state::<PresetUndo>();
    let restored = handle
        .take()
        .ok_or_else(|| "there is nothing to undo".to_owned())?;

    // Taken rather than borrowed, and nothing is held in its place: an undo
    // that leaves an undo behind is a button that redoes on the second press,
    // under the same label.
    adopt(app, state, restored.clone());
    Ok(restored)
}

/// Writes a whole new config out and makes the shell follow it.
fn adopt(app: &AppHandle, state: &AppState, updated: bw_core::Config) {
    if let Err(error) = bw_core::config::save(state.config_path(), &updated) {
        tracing::warn!(%error, "could not write the config a preset produced");
    }

    // Everything a wholesale config change invalidates — the palette, the
    // hotkeys, the hot corners, the desktop wallpaper — is the watcher's list,
    // used here rather than copied. A second copy would drift the first time
    // something was added to it, and the symptom would be a setting that works
    // when edited in the file and not when a preset sets it.
    crate::services::config::adopt(app, state, updated);
}

/// Turns an edited config tree back into a `Config`, refusing anything the
/// schema would not accept.
fn to_config(json: Value, state: &AppState) -> Result<bw_core::Config, PresetError> {
    serde_json::from_value(json).map_err(|source| {
        PresetError::Config(bw_core::config::ConfigError::Parse {
            path: state.config_path().to_owned(),
            source,
        })
    })
}
