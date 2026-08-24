//! System readings, pushed to the surfaces on a timer.
//!
//! Each of these replaces a Quickshell service that read a Linux interface:
//! `/proc` for resources, UPower for the battery, MPRIS for media, Hyprland IPC
//! for workspaces.

use serde::Serialize;

/// CPU, memory and disk, from `sysinfo` rather than `/proc`.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceReading {
    pub cpu: f32,
    pub memory: f32,
    pub memory_used_bytes: u64,
    pub memory_total_bytes: u64,
    pub swap: f32,
    pub disk: f32,
    pub disk_used_bytes: u64,
    pub disk_total_bytes: u64,
}

/// Samples the machine. Holds its own `System` so CPU usage has a previous
/// reading to compare against — a fresh one always reports zero.
pub struct Resources {
    system: sysinfo::System,
    disks: sysinfo::Disks,
}

impl Default for Resources {
    fn default() -> Self {
        Self::new()
    }
}

impl Resources {
    pub fn new() -> Self {
        Self {
            system: sysinfo::System::new(),
            disks: sysinfo::Disks::new_with_refreshed_list(),
        }
    }

    pub fn sample(&mut self) -> ResourceReading {
        self.system.refresh_memory();
        self.system.refresh_cpu_usage();
        self.disks.refresh(true);

        let memory_total = self.system.total_memory();
        let memory_used = self.system.used_memory();
        let swap_total = self.system.total_swap();

        // Only the drive Windows is installed on is interesting for a desktop
        // widget; summing every mounted volume would include removable media.
        let (disk_total, disk_available) = self
            .disks
            .list()
            .iter()
            .find(|disk| disk.mount_point() == std::path::Path::new("C:\\"))
            .or_else(|| self.disks.list().first())
            .map(|disk| (disk.total_space(), disk.available_space()))
            .unwrap_or((0, 0));
        let disk_used = disk_total.saturating_sub(disk_available);

        ResourceReading {
            cpu: self.system.global_cpu_usage(),
            memory: percent(memory_used, memory_total),
            memory_used_bytes: memory_used,
            memory_total_bytes: memory_total,
            swap: percent(self.system.used_swap(), swap_total),
            disk: percent(disk_used, disk_total),
            disk_used_bytes: disk_used,
            disk_total_bytes: disk_total,
        }
    }
}

fn percent(used: u64, total: u64) -> f32 {
    if total == 0 {
        return 0.0;
    }
    (used as f64 / total as f64 * 100.0) as f32
}

/// Battery, from `GetSystemPowerStatus`.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BatteryState {
    pub present: bool,
    pub percent: f32,
    pub charging: bool,
    pub seconds_remaining: Option<u32>,
}

#[cfg(windows)]
pub fn battery() -> BatteryState {
    use windows::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};

    let mut status = SYSTEM_POWER_STATUS::default();
    if unsafe { GetSystemPowerStatus(&mut status) }.is_err() {
        return BatteryState::default();
    }

    // 255 means "unknown" in both of these fields, and BATTERY_FLAG_NO_BATTERY
    // is 128.
    let present = status.BatteryLifePercent != 255 && status.BatteryFlag & 128 == 0;
    let remaining = match status.BatteryLifeTime {
        u32::MAX => None,
        seconds => Some(seconds),
    };

    BatteryState {
        present,
        percent: if present {
            f32::from(status.BatteryLifePercent)
        } else {
            0.0
        },
        // ACLineStatus is 1 when plugged in.
        charging: status.ACLineStatus == 1,
        seconds_remaining: remaining,
    }
}

#[cfg(not(windows))]
pub fn battery() -> BatteryState {
    BatteryState::default()
}

/// Now playing, from the Windows media session — the SMTC analogue of MPRIS.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaState {
    pub playing: bool,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub position: f64,
    pub duration: f64,
    pub artwork: String,
    pub source: String,
}

/// What the media buttons can ask for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaAction {
    PlayPause,
    Next,
    Previous,
}

impl MediaAction {
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "playPause" => Some(Self::PlayPause),
            "next" => Some(Self::Next),
            "previous" => Some(Self::Previous),
            _ => None,
        }
    }
}

#[cfg(windows)]
pub fn media() -> MediaState {
    use windows::Media::Control::{
        GlobalSystemMediaTransportControlsSessionManager as SessionManager,
        GlobalSystemMediaTransportControlsSessionPlaybackStatus as PlaybackStatus,
    };

    // Every step here can legitimately fail — nothing is playing, the session
    // vanished mid-call — so a default reading is the honest answer, not an error.
    let Ok(manager) = SessionManager::RequestAsync().and_then(|operation| operation.get()) else {
        return MediaState::default();
    };
    let Ok(session) = manager.GetCurrentSession() else {
        return MediaState::default();
    };

    let mut state = MediaState {
        source: session
            .SourceAppUserModelId()
            .map(|id| id.to_string())
            .unwrap_or_default(),
        ..MediaState::default()
    };

    if let Ok(properties) = session
        .TryGetMediaPropertiesAsync()
        .and_then(|operation| operation.get())
    {
        state.title = properties
            .Title()
            .map(|t| t.to_string())
            .unwrap_or_default();
        state.artist = properties
            .Artist()
            .map(|a| a.to_string())
            .unwrap_or_default();
        state.album = properties
            .AlbumTitle()
            .map(|a| a.to_string())
            .unwrap_or_default();
    }

    if let Ok(info) = session.GetPlaybackInfo() {
        state.playing = info.PlaybackStatus() == Ok(PlaybackStatus::Playing);
    }

    if let Ok(timeline) = session.GetTimelineProperties() {
        // WinRT TimeSpans are in 100 ns ticks.
        const TICKS_PER_SECOND: f64 = 10_000_000.0;
        state.position = timeline
            .Position()
            .map(|span| span.Duration as f64 / TICKS_PER_SECOND)
            .unwrap_or_default();
        state.duration = timeline
            .EndTime()
            .map(|span| span.Duration as f64 / TICKS_PER_SECOND)
            .unwrap_or_default();
    }

    state
}

#[cfg(not(windows))]
pub fn media() -> MediaState {
    MediaState::default()
}

#[cfg(windows)]
pub fn media_command(action: MediaAction) -> Result<(), String> {
    use windows::Media::Control::GlobalSystemMediaTransportControlsSessionManager as SessionManager;

    let session = SessionManager::RequestAsync()
        .and_then(|operation| operation.get())
        .and_then(|manager| manager.GetCurrentSession())
        .map_err(|error| format!("no media session: {error}"))?;

    let result = match action {
        MediaAction::PlayPause => session.TryTogglePlayPauseAsync().and_then(|op| op.get()),
        MediaAction::Next => session.TrySkipNextAsync().and_then(|op| op.get()),
        MediaAction::Previous => session.TrySkipPreviousAsync().and_then(|op| op.get()),
    };

    result
        .map(|_| ())
        .map_err(|error| format!("the media session refused the command: {error}"))
}

#[cfg(not(windows))]
pub fn media_command(_action: MediaAction) -> Result<(), String> {
    Ok(())
}

/// Workspaces, from whichever tiling window manager is running.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Workspace {
    pub name: String,
    pub display_name: String,
    pub focused: bool,
    pub populated: bool,
    pub monitor: String,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceState {
    /// `"glazewm"`, `"komorebi"`, or `None` when neither is running.
    pub source: Option<String>,
    pub workspaces: Vec<Workspace>,
}

/// Reads workspaces from GlazeWM's WebSocket IPC.
///
/// Windows has no public virtual-desktop API worth building on — the COM
/// interface changes between builds — so the shell integrates with the tiling
/// window managers people actually pair this kind of bar with, and shows nothing
/// when neither is present.
pub async fn workspaces(port: u16) -> WorkspaceState {
    match query_glazewm(port).await {
        Ok(state) => state,
        Err(error) => {
            tracing::debug!(%error, "no GlazeWM workspaces");
            WorkspaceState::default()
        }
    }
}

async fn query_glazewm(port: u16) -> Result<WorkspaceState, String> {
    use futures_util::{SinkExt, StreamExt};

    let (mut socket, _) = tokio_tungstenite::connect_async(format!("ws://127.0.0.1:{port}"))
        .await
        .map_err(|error| format!("could not connect to GlazeWM: {error}"))?;

    socket
        .send(tokio_tungstenite::tungstenite::Message::Text(
            "query workspaces".into(),
        ))
        .await
        .map_err(|error| format!("could not query GlazeWM: {error}"))?;

    let message = socket
        .next()
        .await
        .ok_or_else(|| "GlazeWM closed the connection".to_owned())?
        .map_err(|error| format!("GlazeWM sent nothing usable: {error}"))?;

    let body = message.into_text().map_err(|error| error.to_string())?;
    parse_glazewm(&body)
}

fn parse_glazewm(body: &str) -> Result<WorkspaceState, String> {
    let root: serde_json::Value = serde_json::from_str(body)
        .map_err(|error| format!("GlazeWM sent invalid JSON: {error}"))?;

    let list = root
        .pointer("/data/workspaces")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "GlazeWM sent no workspace list".to_owned())?;

    let workspaces = list
        .iter()
        .map(|entry| {
            let name = entry
                .get("name")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_owned();
            Workspace {
                display_name: entry
                    .get("displayName")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or(&name)
                    .to_owned(),
                focused: entry
                    .get("hasFocus")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false),
                populated: entry
                    .get("children")
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(|children| !children.is_empty()),
                monitor: entry
                    .get("parent")
                    .and_then(|parent| parent.get("name"))
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
                    .to_owned(),
                name,
            }
        })
        .collect();

    Ok(WorkspaceState {
        source: Some("glazewm".to_owned()),
        workspaces,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percentages_never_divide_by_zero() {
        assert_eq!(percent(0, 0), 0.0);
        assert_eq!(percent(5, 0), 0.0);
        assert!((percent(1, 2) - 50.0).abs() < f32::EPSILON);
    }

    #[test]
    fn media_actions_parse_from_their_wire_names() {
        assert_eq!(
            MediaAction::parse("playPause"),
            Some(MediaAction::PlayPause)
        );
        assert_eq!(MediaAction::parse("next"), Some(MediaAction::Next));
        assert_eq!(MediaAction::parse("previous"), Some(MediaAction::Previous));
        assert_eq!(MediaAction::parse("stop"), None);
    }

    #[test]
    fn glazewm_workspaces_parse() {
        let body = r#"{
          "data": { "workspaces": [
            { "name": "1", "displayName": "one", "hasFocus": true,
              "children": [{}], "parent": { "name": "\\\\.\\DISPLAY1" } },
            { "name": "2", "hasFocus": false, "children": [] }
          ]}
        }"#;
        let state = parse_glazewm(body).unwrap();
        assert_eq!(state.source.as_deref(), Some("glazewm"));
        assert_eq!(state.workspaces.len(), 2);
        assert!(state.workspaces[0].focused);
        assert!(state.workspaces[0].populated);
        assert_eq!(state.workspaces[0].display_name, "one");
        assert_eq!(state.workspaces[0].monitor, r"\\.\DISPLAY1");
        // A workspace with no displayName falls back to its name.
        assert_eq!(state.workspaces[1].display_name, "2");
        assert!(!state.workspaces[1].populated);
    }

    #[test]
    fn a_response_without_workspaces_is_an_error_not_an_empty_bar() {
        assert!(parse_glazewm(r#"{"data":{}}"#).is_err());
        assert!(parse_glazewm("not json").is_err());
    }
}

/// Network throughput, derived from the interface counters.
#[derive(Debug, Clone, Copy, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkReading {
    /// Bytes per second, averaged over the interval since the last sample.
    pub down: f64,
    pub up: f64,
    pub total_received: u64,
    pub total_sent: u64,
}

/// Samples throughput. Windows reports cumulative octets, so a rate needs the
/// previous reading and the time between the two.
pub struct Network {
    #[cfg(windows)]
    previous: Option<(crate::platform::win::NetworkCounters, std::time::Instant)>,
}

impl Default for Network {
    fn default() -> Self {
        Self::new()
    }
}

impl Network {
    pub fn new() -> Self {
        Self {
            #[cfg(windows)]
            previous: None,
        }
    }

    #[cfg(windows)]
    pub fn sample(&mut self) -> NetworkReading {
        let counters = crate::platform::win::network_counters();
        let now = std::time::Instant::now();

        let rate = match self.previous {
            // The first sample has nothing to compare against, so it reports the
            // totals and a rate of zero rather than a spike.
            None => (0.0, 0.0),
            Some((before, at)) => {
                let seconds = now.duration_since(at).as_secs_f64();
                if seconds <= 0.0 {
                    (0.0, 0.0)
                } else {
                    (
                        counters.received.saturating_sub(before.received) as f64 / seconds,
                        counters.sent.saturating_sub(before.sent) as f64 / seconds,
                    )
                }
            }
        };

        self.previous = Some((counters, now));
        NetworkReading {
            down: rate.0,
            up: rate.1,
            total_received: counters.received,
            total_sent: counters.sent,
        }
    }

    #[cfg(not(windows))]
    pub fn sample(&mut self) -> NetworkReading {
        NetworkReading::default()
    }
}

/// The window the user is working in, for the bar's active-window widget.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActiveWindow {
    pub title: String,
    pub class: String,
}

#[cfg(windows)]
pub fn active_window() -> ActiveWindow {
    let reading = crate::platform::win::active_window();
    ActiveWindow {
        title: reading.title,
        class: reading.class,
    }
}

#[cfg(not(windows))]
pub fn active_window() -> ActiveWindow {
    ActiveWindow::default()
}

/// The notification area's icons.
#[cfg(windows)]
pub fn tray_icons() -> Vec<crate::platform::tray::TrayIcon> {
    crate::platform::tray::icons()
}

#[cfg(not(windows))]
pub fn tray_icons() -> Vec<serde_json::Value> {
    Vec::new()
}
