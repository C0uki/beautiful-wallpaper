//! The commands surfaces call, and the events they get back.
//!
//! Names are shared with the frontend through `packages/core/src/ipc.ts`; the
//! IPC target vocabulary matches end4-pC's `IpcHandler` targets so scripts and
//! muscle memory carry over.

use bw_core::{wallpaper::online::WallpaperPage, Config, GeneratedTheme};
use serde_json::Value;
use tauri::{AppHandle, Emitter, State};

use crate::providers::{self, MediaAction};
use crate::services;
use crate::state::{AppState, GlobalStates};

/// Event names, mirrored in `packages/core/src/ipc.ts`.
pub mod event {
    pub const CONFIG_CHANGED: &str = "bw://config-changed";
    pub const THEME_CHANGED: &str = "bw://theme-changed";
    pub const WALLPAPER_CHANGED: &str = "bw://wallpaper-changed";
    pub const STATE_CHANGED: &str = "bw://state-changed";
    pub const RESOURCES: &str = "bw://resources";
    pub const MEDIA: &str = "bw://media";
    pub const BATTERY: &str = "bw://battery";
    pub const WEATHER: &str = "bw://weather";
    pub const WORKSPACES: &str = "bw://workspaces";
    pub const ACTIVE_WINDOW: &str = "bw://active-window";
    pub const NETWORK: &str = "bw://network";
    pub const TRAY: &str = "bw://tray";
}

#[tauri::command]
pub fn get_config(state: State<'_, AppState>) -> Config {
    state.config()
}

#[tauri::command]
pub fn set_config_value(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
    value: Value,
) -> Result<Config, String> {
    let updated = state
        .set_config_value(&path, value)
        .map_err(|error| error.to_string())?;
    let _ = app.emit(event::CONFIG_CHANGED, &updated);

    // Anything under `appearance` can change the palette, so regenerate rather
    // than leaving the shell showing colours the config no longer describes.
    if path.starts_with("appearance.") {
        if let Ok(theme) = services::theme::regenerate(&state) {
            let _ = app.emit(event::THEME_CHANGED, &theme);
        }
    }

    Ok(updated)
}

#[tauri::command]
pub fn get_theme(state: State<'_, AppState>) -> Result<GeneratedTheme, String> {
    match state.theme() {
        Some(theme) => Ok(theme),
        None => services::theme::regenerate(&state),
    }
}

#[tauri::command]
pub fn set_mode(app: AppHandle, state: State<'_, AppState>, mode: String) -> Result<(), String> {
    state
        .set_config_value("appearance.palette.mode", Value::String(mode))
        .map_err(|error| error.to_string())?;

    let theme = services::theme::regenerate(&state)?;
    let _ = app.emit(event::THEME_CHANGED, &theme);
    let _ = app.emit(event::CONFIG_CHANGED, &state.config());
    Ok(())
}

#[tauri::command]
pub fn get_states(state: State<'_, AppState>) -> GlobalStates {
    state.states()
}

#[tauri::command]
pub fn toggle_state(
    app: AppHandle,
    state: State<'_, AppState>,
    name: String,
) -> Result<GlobalStates, String> {
    let states = state
        .toggle_state(&name)
        .ok_or_else(|| format!("there is no surface flag called `{name}`"))?;
    let _ = app.emit(event::STATE_CHANGED, &states);
    Ok(states)
}

#[tauri::command]
pub fn set_state(
    app: AppHandle,
    state: State<'_, AppState>,
    name: String,
    value: bool,
) -> Result<GlobalStates, String> {
    let states = state
        .set_state(&name, value)
        .ok_or_else(|| format!("there is no surface flag called `{name}`"))?;
    let _ = app.emit(event::STATE_CHANGED, &states);
    Ok(states)
}

#[tauri::command]
pub fn list_wallpapers(
    state: State<'_, AppState>,
    path: Option<String>,
) -> Result<Vec<bw_core::wallpaper::Entry>, String> {
    services::wallpaper::list(&state, path.as_deref())
}

#[tauri::command]
pub fn apply_wallpaper(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> Result<(), String> {
    services::wallpaper::apply(&state, &path)?;
    let theme = services::theme::regenerate(&state)?;

    let _ = app.emit(
        event::WALLPAPER_CHANGED,
        serde_json::json!({ "monitor": "", "path": path, "blanked": false }),
    );
    let _ = app.emit(event::THEME_CHANGED, &theme);
    let _ = app.emit(event::CONFIG_CHANGED, &state.config());
    Ok(())
}

#[tauri::command]
pub fn random_wallpaper(app: AppHandle, state: State<'_, AppState>) -> Result<String, String> {
    let path = services::wallpaper::random(&state)?;
    let theme = services::theme::regenerate(&state)?;

    let _ = app.emit(
        event::WALLPAPER_CHANGED,
        serde_json::json!({ "monitor": "", "path": path, "blanked": false }),
    );
    let _ = app.emit(event::THEME_CHANGED, &theme);
    let _ = app.emit(event::CONFIG_CHANGED, &state.config());
    Ok(path)
}

#[tauri::command]
pub fn thumbnail_for(path: String, size: Option<u32>) -> Result<String, String> {
    let file = services::wallpaper::thumbnail(&path, size.unwrap_or(480))?;
    Ok(file.to_string_lossy().into_owned())
}

#[tauri::command]
pub async fn search_online_wallpapers(
    state: State<'_, AppState>,
    provider: String,
    search: String,
    page: u32,
) -> Result<WallpaperPage, String> {
    services::wallpaper::search_online(&state, &provider, &search, page).await
}

#[tauri::command]
pub async fn download_wallpaper(
    state: State<'_, AppState>,
    url: String,
    provider: String,
    download_location: Option<String>,
) -> Result<String, String> {
    services::wallpaper::download(
        &state,
        &url,
        &provider,
        download_location.as_deref().unwrap_or_default(),
    )
    .await
}

#[tauri::command]
pub fn set_api_key(provider: String, key: String) -> Result<(), String> {
    services::wallpaper::set_api_key(&provider, &key)
}

/// The monitors, so a surface can position itself against the same geometry the
/// backend used.
#[tauri::command]
pub fn get_monitors() -> Vec<MonitorInfo> {
    monitors()
}

#[cfg(windows)]
type MonitorInfo = crate::platform::win::Monitor;

#[cfg(not(windows))]
type MonitorInfo = serde_json::Value;

#[cfg(windows)]
fn monitors() -> Vec<MonitorInfo> {
    crate::platform::win::monitors()
}

#[cfg(not(windows))]
fn monitors() -> Vec<MonitorInfo> {
    Vec::new()
}

/// Shows or hides the stock Windows taskbar.
///
/// Only ever called from the settings toggle; the shell never does this on its
/// own, and restores the taskbar when the setting is turned back off.
#[tauri::command]
pub fn set_taskbar_visible(_visible: bool) -> Result<(), String> {
    #[cfg(windows)]
    unsafe {
        crate::platform::win::set_taskbar_visible(_visible);
    }
    Ok(())
}

#[tauri::command]
pub fn media_command(action: String) -> Result<(), String> {
    let action =
        MediaAction::parse(&action).ok_or_else(|| format!("unknown media action `{action}`"))?;
    providers::media_command(action)
}
