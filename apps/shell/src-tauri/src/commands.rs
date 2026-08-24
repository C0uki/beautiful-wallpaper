//! The commands surfaces call, and the events they get back.
//!
//! Names are shared with the frontend through `packages/core/src/ipc.ts`; the
//! IPC target vocabulary matches end4-pC's `IpcHandler` targets so scripts and
//! muscle memory carry over.

use bw_core::{
    wallpaper::online::WallpaperPage, Config, GeneratedTheme, NewNotification, Notification,
    Urgency,
};
use serde_json::Value;
use tauri::{AppHandle, Emitter, State};

use crate::providers::{self, MediaAction};
use crate::services;
use crate::state::{AppState, GlobalStates, NotificationStore, VolumeHandle};

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
    pub const NOTIFICATIONS: &str = "bw://notifications";
    pub const VOLUME: &str = "bw://volume";
    pub const BRIGHTNESS: &str = "bw://brightness";
    /// Asks the readout surface to appear, carrying what to show.
    pub const OSD: &str = "bw://osd";
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
    crate::surfaces::apply_states(&app, &states);
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
    crate::surfaces::apply_states(&app, &states);
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

// --- Notifications ---------------------------------------------------------

/// The notification history, newest first.
#[tauri::command]
pub fn get_notifications(store: State<'_, NotificationStore>) -> Vec<Notification> {
    store.0.list()
}

/// Records a notification and shows it. Used by the shell's own actions today,
/// and by a notification listener once the shell has package identity.
#[tauri::command]
pub fn post_notification(
    app: AppHandle,
    store: State<'_, NotificationStore>,
    summary: String,
    body: Option<String>,
    app_name: Option<String>,
    urgency: Option<String>,
) -> Notification {
    let notification = store.0.post(NewNotification {
        app_name: app_name.unwrap_or_else(|| "beautiful-wallpaper".to_owned()),
        summary,
        body: body.unwrap_or_default(),
        urgency: parse_urgency(urgency.as_deref()),
        ..NewNotification::default()
    });
    let _ = app.emit(event::NOTIFICATIONS, store.0.list());
    notification
}

#[tauri::command]
pub fn dismiss_notification(app: AppHandle, store: State<'_, NotificationStore>, id: u32) {
    if store.0.dismiss(id) {
        let _ = app.emit(event::NOTIFICATIONS, store.0.list());
    }
}

#[tauri::command]
pub fn clear_notifications(app: AppHandle, store: State<'_, NotificationStore>) {
    store.0.clear();
    let _ = app.emit(event::NOTIFICATIONS, store.0.list());
}

fn parse_urgency(name: Option<&str>) -> Urgency {
    match name {
        Some("low") => Urgency::Low,
        Some("critical") => Urgency::Critical,
        _ => Urgency::Normal,
    }
}

// --- Volume ----------------------------------------------------------------

/// The current output level, for a surface that has just opened.
#[tauri::command]
pub fn get_volume(volume: State<'_, VolumeHandle>) -> providers::VolumeReading {
    volume.read()
}

#[tauri::command]
pub fn set_volume(
    state: State<'_, AppState>,
    volume: State<'_, VolumeHandle>,
    percent: f32,
) -> Result<(), String> {
    let ceiling = hearing_ceiling(&state.config());
    volume.set_percent(percent, ceiling)
}

#[tauri::command]
pub fn set_muted(volume: State<'_, VolumeHandle>, muted: bool) -> Result<(), String> {
    volume.set_muted(muted)
}

/// Moves the volume by one configured step, clamped to the usable range.
#[tauri::command]
pub fn step_volume(
    state: State<'_, AppState>,
    volume: State<'_, VolumeHandle>,
    up: bool,
) -> Result<(), String> {
    let config = state.config();
    let step = config.audio.step as f32;
    let ceiling = hearing_ceiling(&config);

    let current = volume.read().percent;
    let target = if up { current + step } else { current - step };
    volume.set_percent(target.clamp(0.0, 100.0), ceiling)
}

/// The highest volume the shell's own controls will set.
fn hearing_ceiling(config: &Config) -> f32 {
    if config.audio.protection.enable {
        config.audio.protection.max_volume as f32
    } else {
        100.0
    }
}
