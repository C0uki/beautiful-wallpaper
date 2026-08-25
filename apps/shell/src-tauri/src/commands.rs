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
use crate::state::{
    AppState, BrightnessHandle, GlobalStates, IdleHandle, MicHandle, MixerHandle,
    NotificationStore, PersistentStore, TodoStore, VolumeHandle,
};

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
    pub const TODOS: &str = "bw://todos";
    /// Runtime state that is not configuration.
    pub const PERSISTENT: &str = "bw://persistent";
    pub const VOLUME: &str = "bw://volume";
    pub const MIC: &str = "bw://mic";
    /// The per-application mixer changed: a session appeared, went away, or
    /// moved its own level.
    pub const AUDIO_SESSIONS: &str = "bw://audio-sessions";
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

// --- Brightness ------------------------------------------------------------

/// What a surface needs to decide whether to draw the control at all.
#[derive(Debug, Clone, Copy, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BrightnessReading {
    /// 0–100, or `null` when no display can report a level.
    pub percent: Option<u8>,
    pub supported: bool,
}

#[tauri::command]
pub fn get_brightness(brightness: State<'_, BrightnessHandle>) -> BrightnessReading {
    let percent = brightness.read();
    BrightnessReading {
        percent,
        supported: percent.is_some(),
    }
}

#[tauri::command]
pub fn set_brightness(brightness: State<'_, BrightnessHandle>, percent: u8) {
    brightness.set(percent.min(100));
}

/// Moves brightness by one step, the way the volume keys move volume.
///
/// Does nothing when the display cannot report a level: stepping from an
/// unknown starting point would jump the backlight somewhere arbitrary.
#[tauri::command]
pub fn step_brightness(brightness: State<'_, BrightnessHandle>, up: bool) {
    let Some(current) = brightness.read() else {
        return;
    };
    let step = BRIGHTNESS_STEP;
    let target = if up {
        current.saturating_add(step)
    } else {
        current.saturating_sub(step)
    };
    brightness.set(target.min(100));
}

/// Percentage points per brightness step. Matches the audio default, so the
/// two sets of media keys feel the same.
const BRIGHTNESS_STEP: u8 = 5;

/// Turns the night light on at the configured temperature, or off.
#[tauri::command]
pub fn set_night_light(
    state: State<'_, AppState>,
    brightness: State<'_, BrightnessHandle>,
    enable: bool,
) -> Result<Config, String> {
    let config = state.config();
    let kelvin = enable.then_some(config.sidebar.night_light.temperature);
    brightness.set_night_light(kelvin);

    // The toggle is part of the config, so it survives a restart the way the
    // original's does.
    state
        .set_config_value("sidebar.nightLight.enable", serde_json::json!(enable))
        .map_err(|error| error.to_string())
}

// --- Microphone and the mixer ----------------------------------------------

#[tauri::command]
pub fn get_mic(mic: State<'_, MicHandle>) -> providers::VolumeReading {
    mic.0.read()
}

/// The microphone has no hearing-protection ceiling: turning an input up
/// cannot hurt anybody's ears.
#[tauri::command]
pub fn set_mic(mic: State<'_, MicHandle>, percent: f32) -> Result<(), String> {
    mic.0.set_percent(percent.clamp(0.0, 100.0), 100.0)
}

#[tauri::command]
pub fn set_mic_muted(mic: State<'_, MicHandle>, muted: bool) -> Result<(), String> {
    mic.0.set_muted(muted)
}

/// Every application currently playing audio.
#[cfg(windows)]
#[tauri::command]
pub fn get_audio_sessions(
    mixer: State<'_, MixerHandle>,
) -> Vec<crate::platform::mixer::SessionInfo> {
    mixer.list()
}

#[cfg(not(windows))]
#[tauri::command]
pub fn get_audio_sessions(mixer: State<'_, MixerHandle>) -> Vec<()> {
    mixer.list()
}

#[tauri::command]
pub fn set_session_volume(
    state: State<'_, AppState>,
    mixer: State<'_, MixerHandle>,
    id: String,
    percent: f32,
) -> Result<(), String> {
    // The same ceiling the master volume respects: a per-application slider is
    // no less able to hurt somebody wearing headphones.
    let ceiling = hearing_ceiling(&state.config());
    mixer.set_percent(&id, percent.clamp(0.0, 100.0), ceiling)
}

#[tauri::command]
pub fn set_session_muted(
    mixer: State<'_, MixerHandle>,
    id: String,
    muted: bool,
) -> Result<(), String> {
    mixer.set_muted(&id, muted)
}

// --- Radios, idle and the banner -------------------------------------------

/// The WinRT calls behind these block for as long as the radio stack takes —
/// a Wi-Fi scan is measured in seconds — so each runs on a blocking thread
/// rather than stalling the async runtime the whole shell shares.
async fn off_runtime<T, F>(work: F) -> T
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    tokio::task::spawn_blocking(work)
        .await
        .unwrap_or_else(|_| unreachable!("the blocking pool does not panic here"))
}

#[tauri::command]
pub async fn get_radios() -> providers::RadiosState {
    #[cfg(windows)]
    {
        off_runtime(crate::platform::radios::state).await
    }
    #[cfg(not(windows))]
    providers::RadiosState::default()
}

#[tauri::command]
pub async fn set_radio(_kind: String, _on: bool) -> bool {
    #[cfg(windows)]
    {
        let Some(kind) = crate::platform::radios::Kind::parse(&_kind) else {
            return false;
        };
        off_runtime(move || crate::platform::radios::set(kind, _on)).await
    }
    #[cfg(not(windows))]
    false
}

#[tauri::command]
pub async fn scan_wifi() -> Vec<providers::WifiNetwork> {
    #[cfg(windows)]
    {
        off_runtime(crate::platform::radios::scan).await
    }
    #[cfg(not(windows))]
    Vec::new()
}

#[tauri::command]
pub async fn connect_wifi(_ssid: String, _password: Option<String>) -> providers::ConnectOutcome {
    #[cfg(windows)]
    {
        off_runtime(move || crate::platform::radios::connect(&_ssid, _password.as_deref())).await
    }
    #[cfg(not(windows))]
    providers::ConnectOutcome::Failed
}

#[tauri::command]
pub async fn disconnect_wifi() {
    #[cfg(windows)]
    off_runtime(crate::platform::radios::disconnect).await;
}

#[tauri::command]
pub async fn get_bluetooth_devices() -> Vec<providers::BluetoothDeviceInfo> {
    #[cfg(windows)]
    {
        off_runtime(crate::platform::radios::paired_devices).await
    }
    #[cfg(not(windows))]
    Vec::new()
}

#[tauri::command]
pub fn get_idle_inhibit(idle: State<'_, IdleHandle>) -> bool {
    idle.is_on()
}

#[tauri::command]
pub fn set_idle_inhibit(idle: State<'_, IdleHandle>, on: bool) -> bool {
    idle.set(on);
    idle.is_on()
}

/// What the sidebar's banner shows about the machine.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemInfo {
    pub username: String,
    pub hostname: String,
    /// Already formatted: the exact wording is shared with the tests in
    /// `bw_core::sysinfo`, and a raw second count would only be reformatted
    /// identically in three different surfaces.
    pub uptime: String,
}

#[tauri::command]
pub fn get_system_info(state: State<'_, AppState>) -> SystemInfo {
    let config = state.config();

    // `USERNAME` is set for every interactive session, and reading it avoids a
    // Win32 call for something the environment already knows.
    let username = std::env::var("USERNAME")
        .ok()
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "user".to_owned());

    SystemInfo {
        username: if config.sidebar.profile.display_name.is_empty() {
            username
        } else {
            config.sidebar.profile.display_name.clone()
        },
        hostname: sysinfo::System::host_name().unwrap_or_default(),
        uptime: bw_core::sysinfo::format_uptime(sysinfo::System::uptime()),
    }
}

// --- To do, and persistent state -------------------------------------------

/// Emits the list after any change, so every surface showing it agrees.
fn publish_todos(app: &AppHandle, store: &TodoStore) {
    let _ = app.emit(event::TODOS, store.0.list());
}

#[tauri::command]
pub fn get_todos(store: State<'_, TodoStore>) -> Vec<bw_core::TodoItem> {
    store.0.list()
}

/// Adds a task. Returns the whole list rather than the new item: the caller
/// wants to redraw, and a blank or over-full add returns nothing to append.
#[tauri::command]
pub fn add_todo(
    app: AppHandle,
    store: State<'_, TodoStore>,
    content: String,
) -> Vec<bw_core::TodoItem> {
    store.0.add(content);
    let list = store.0.list();
    publish_todos(&app, &store);
    list
}

#[tauri::command]
pub fn set_todo_done(
    app: AppHandle,
    store: State<'_, TodoStore>,
    id: u32,
    done: bool,
) -> Vec<bw_core::TodoItem> {
    store.0.set_done(id, done);
    let list = store.0.list();
    publish_todos(&app, &store);
    list
}

#[tauri::command]
pub fn remove_todo(app: AppHandle, store: State<'_, TodoStore>, id: u32) -> Vec<bw_core::TodoItem> {
    store.0.remove(id);
    let list = store.0.list();
    publish_todos(&app, &store);
    list
}

#[tauri::command]
pub fn clear_done_todos(app: AppHandle, store: State<'_, TodoStore>) -> Vec<bw_core::TodoItem> {
    store.0.clear_done();
    let list = store.0.list();
    publish_todos(&app, &store);
    list
}

#[tauri::command]
pub fn reorder_todo(
    app: AppHandle,
    store: State<'_, TodoStore>,
    id: u32,
    to: usize,
) -> Vec<bw_core::TodoItem> {
    store.0.reorder(id, to);
    let list = store.0.list();
    publish_todos(&app, &store);
    list
}

#[tauri::command]
pub fn get_persistent(store: State<'_, PersistentStore>) -> bw_core::Persistent {
    store.0.get()
}

/// Applies a dotted-path edit, the same vocabulary `set_config_value` uses.
#[tauri::command]
pub fn set_persistent_value(
    app: AppHandle,
    store: State<'_, PersistentStore>,
    path: String,
    value: serde_json::Value,
) -> Result<bw_core::Persistent, String> {
    let updated = store
        .0
        .set_path(&path, value)
        .map_err(|error| error.to_string())?;
    let _ = app.emit(event::PERSISTENT, &updated);
    Ok(updated)
}
