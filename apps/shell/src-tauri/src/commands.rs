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
use tauri::{AppHandle, Emitter, Manager, State};

use crate::providers::{self, MediaAction};
use crate::services;
use crate::state::{
    AppState, BrightnessHandle, ChatBusy, ChatStore, DockHandle, GlobalStates, IdleHandle,
    MicHandle, MixerHandle, NotificationStore, PersistentStore, TodoStore, VolumeHandle,
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
    /// The dock's icons changed: a window opened, closed or came forward.
    pub const DOCK: &str = "bw://dock";
    /// The whole conversation, after a turn starts or finishes.
    pub const CHAT: &str = "bw://chat";
    /// One piece of a reply as it streams. Separate from `CHAT` so a token
    /// does not redraw every message in the window.
    pub const CHAT_EVENT: &str = "bw://chat-event";
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

// --- The dock --------------------------------------------------------------

#[tauri::command]
pub fn get_dock_items(
    state: State<'_, AppState>,
    dock: State<'_, DockHandle>,
) -> Vec<bw_core::dock::DockApp> {
    dock.items(&state.config())
}

/// What happened when the dock was clicked, so the UI can say something useful
/// rather than looking inert.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ActivateOutcome {
    Activated,
    Minimised,
    /// Windows refused to move the foreground; the window was flashed instead.
    Flashed,
    /// The window has gone since the dock last looked.
    Gone,
}

/// Raises a window, or minimises it when it is already at the front.
///
/// That toggle is what a taskbar does, and doing anything else makes clicking
/// the icon of the program you are already in feel broken.
#[tauri::command]
pub fn activate_window(_id: String, _minimise_if_active: bool) -> ActivateOutcome {
    #[cfg(windows)]
    {
        use crate::platform::windows;

        let Some(window) = windows::parse_id(&_id) else {
            return ActivateOutcome::Gone;
        };

        if _minimise_if_active && !windows::is_minimised(window) {
            windows::minimise(window);
            return ActivateOutcome::Minimised;
        }

        if windows::activate(window) {
            return ActivateOutcome::Activated;
        }
        // Refused. Flashing is what Explorer does in the same situation.
        windows::flash(window);
        ActivateOutcome::Flashed
    }
    #[cfg(not(windows))]
    ActivateOutcome::Gone
}

/// Starts a pinned application.
#[tauri::command]
pub fn launch_app(app: AppHandle, path: String) -> Result<(), String> {
    tauri_plugin_opener::open_path(&path, None::<&str>)
        .map_err(|error| format!("could not start {path}: {error}"))?;
    let _ = app;
    Ok(())
}

/// Adds or removes a pinned application, returning the updated config.
#[tauri::command]
pub fn set_pinned(
    state: State<'_, AppState>,
    path: String,
    pinned: bool,
) -> Result<Config, String> {
    let mut pins = state.config().dock.pinned_apps;
    // Paths are compared the way the dock groups them, so pinning an
    // application that is already pinned under different casing is a no-op
    // rather than a duplicate icon.
    let key = path.replace('/', "\\").to_lowercase();
    pins.retain(|existing| existing.replace('/', "\\").to_lowercase() != key);
    if pinned {
        pins.push(path);
    }

    state
        .set_config_value("dock.pinnedApps", serde_json::json!(pins))
        .map_err(|error| error.to_string())
}

// --- Translation -----------------------------------------------------------

/// Whether an API key has been configured, so the sidebar can show the
/// translator or a pointer at the settings rather than a dead tab.
#[tauri::command]
pub fn has_ai_key() -> bool {
    crate::services::ai::has_key()
}

#[tauri::command]
pub fn set_ai_key(key: String) -> Result<(), String> {
    crate::services::ai::set_key(&key)
}

/// The translator's result: the text, or a reason the UI can act on.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslationResult {
    pub text: String,
    pub error: Option<bw_core::ai::AiError>,
}

#[tauri::command]
pub async fn translate(
    state: State<'_, AppState>,
    text: String,
    from: String,
    to: String,
) -> Result<TranslationResult, String> {
    let config = state.config();
    match crate::services::ai::translate(&config, &text, &from, &to).await {
        Ok(text) => Ok(TranslationResult { text, error: None }),
        Err(error) => Ok(TranslationResult {
            text: String::new(),
            error: Some(error),
        }),
    }
}

// --- The chat --------------------------------------------------------------

#[tauri::command]
pub fn get_chat(store: State<'_, ChatStore>) -> Vec<bw_core::chat::ChatMessage> {
    store.0.list()
}

#[tauri::command]
pub fn clear_chat(app: AppHandle, store: State<'_, ChatStore>) -> Vec<bw_core::chat::ChatMessage> {
    store.0.clear();
    let _ = app.emit(event::CHAT, Vec::<bw_core::chat::ChatMessage>::new());
    Vec::new()
}

/// Sends a message and streams the reply.
///
/// Returns as soon as the request is under way; the reply arrives through
/// `bw://chat` so the window paints as it comes rather than after it.
#[tauri::command]
pub async fn send_chat(
    app: AppHandle,
    text: String,
    attachments: Vec<String>,
) -> Result<(), String> {
    use bw_core::chat::StreamEvent;
    use std::sync::atomic::Ordering;

    if text.trim().is_empty() && attachments.is_empty() {
        return Ok(());
    }

    // One reply at a time. Two streams writing into one conversation would
    // interleave into nonsense, and the history is resent on every request.
    let busy = app.state::<ChatBusy>();
    if busy.0.swap(true, Ordering::SeqCst) {
        return Err("a reply is already on its way".to_owned());
    }

    // Whatever happens below, the flag has to come back down.
    let result = send_chat_inner(&app, text, attachments).await;
    app.state::<ChatBusy>().0.store(false, Ordering::SeqCst);

    if let Err(error) = &result {
        tracing::warn!(%error, "the chat request failed");
        let _ = app.emit(
            event::CHAT_EVENT,
            StreamEvent::Failed(bw_core::ai::AiError::Unavailable),
        );
    }
    result
}

async fn send_chat_inner(
    app: &AppHandle,
    text: String,
    attachments: Vec<String>,
) -> Result<(), String> {
    use bw_core::chat::{Role, StreamEvent};

    // Files are read before the turn is recorded: an unreadable attachment
    // should fail the send rather than leave a half-finished exchange behind.
    let mut files = Vec::new();
    for path in &attachments {
        files.push(crate::services::ai::read_attachment(std::path::Path::new(
            path,
        ))?);
    }
    let names: Vec<String> = files.iter().map(|file| file.name.clone()).collect();

    let store = app.state::<ChatStore>();
    store.0.append(Role::User, text, names);
    let reply = store.0.append(Role::Assistant, String::new(), Vec::new());
    let history = store.0.list();
    let _ = app.emit(event::CHAT, &history);

    // The assistant turn is already in the history as an empty placeholder;
    // sending it back would be an empty message, which the API rejects.
    let request: Vec<bw_core::chat::ChatMessage> = history
        .iter()
        .filter(|message| message.id != reply.id)
        .cloned()
        .collect();

    let config = app.state::<AppState>().config();
    let handle = app.clone();
    let id = reply.id;

    crate::services::ai::stream(&config, &request, &files, move |event| {
        let store = handle.state::<ChatStore>();

        match &event {
            StreamEvent::Text(piece) => {
                store
                    .0
                    .update(id, |message| message.content.push_str(piece));
            }
            StreamEvent::Thinking(piece) => {
                store
                    .0
                    .update(id, |message| message.thinking.push_str(piece));
            }
            StreamEvent::Search(query) => {
                store
                    .0
                    .update(id, |message| message.searches.push(query.clone()));
            }
            StreamEvent::Sources(sources) => {
                store
                    .0
                    .update(id, |message| message.sources.extend(sources.clone()));
            }
            StreamEvent::FellBackTo(model) => {
                store
                    .0
                    .update(id, |message| message.answered_by = model.clone());
            }
            StreamEvent::Done | StreamEvent::Failed(_) => {
                let _ = handle.emit(event::CHAT, store.0.list());
            }
        }

        // The deltas go out on their own channel: re-emitting the whole
        // conversation per token would redraw every message in the window for
        // each character that arrives.
        let _ = handle.emit(event::CHAT_EVENT, &event);
    })
    .await;

    Ok(())
}

/// Drops the last exchange, for retrying after a failure.
#[tauri::command]
pub fn retry_chat(app: AppHandle, store: State<'_, ChatStore>) -> Vec<bw_core::chat::ChatMessage> {
    // Two pops: the empty assistant turn, and the user turn to be resent.
    store.0.pop();
    let list = store.0.list();
    let _ = app.emit(event::CHAT, &list);
    list
}
