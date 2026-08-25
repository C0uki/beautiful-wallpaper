// Names shared by the Rust backend, the surfaces, and the `bw.exe` CLI.
//
// end4-pC addresses its panels through Quickshell `IpcHandler` targets
// (`qs ipc call wallpapers apply <path>`). The same target/function vocabulary is
// kept here so muscle memory and any scripts carry over; on Windows the
// transport underneath is a named pipe rather than the Quickshell socket.

/** Tauri events pushed from Rust to every surface. */
export const Event = {
  /** The whole config, after a file change or a `config.set`. */
  ConfigChanged: "bw://config-changed",
  /** A newly generated Material scheme. */
  ThemeChanged: "bw://theme-changed",
  /** The wallpaper for one monitor changed; carries a `WallpaperChanged`. */
  WallpaperChanged: "bw://wallpaper-changed",
  /** A `GlobalStates` flag was toggled, from a hotkey, the CLI or another surface. */
  StateChanged: "bw://state-changed",
  /** Sampled system readings, on the configured interval. */
  Resources: "bw://resources",
  /** Now-playing information from the Windows media session. */
  Media: "bw://media",
  Battery: "bw://battery",
  Weather: "bw://weather",
  /** Workspaces from GlazeWM or komorebi, or `null` when neither is running. */
  Workspaces: "bw://workspaces",
  /** The window the user is currently working in. */
  ActiveWindow: "bw://active-window",
  /** Network throughput. */
  Network: "bw://network",
  /** The notification area's icons. */
  Tray: "bw://tray",
  /** The notification history, newest first. */
  Notifications: "bw://notifications",
  /** Output level, pushed by WASAPI rather than polled. */
  Volume: "bw://volume",
  Brightness: "bw://brightness",
  /** Microphone level, pushed the same way the output level is. */
  Mic: "bw://mic",
  /** The per-application mixer changed. Carries the whole list. */
  AudioSessions: "bw://audio-sessions",
  /** The to-do list changed. */
  Todos: "bw://todos",
  /** Runtime state that is not configuration — the open tab, the toggle grid. */
  Persistent: "bw://persistent",
  /** Asks the readout to appear, carrying what to show. */
  Osd: "bw://osd",
} as const;

export type EventName = (typeof Event)[keyof typeof Event];

/** Tauri commands callable from any surface. */
export const Command = {
  GetConfig: "get_config",
  SetConfigValue: "set_config_value",
  GetTheme: "get_theme",
  ListWallpapers: "list_wallpapers",
  ApplyWallpaper: "apply_wallpaper",
  RandomWallpaper: "random_wallpaper",
  SearchOnlineWallpapers: "search_online_wallpapers",
  DownloadWallpaper: "download_wallpaper",
  ThumbnailFor: "thumbnail_for",
  SetMode: "set_mode",
  ToggleState: "toggle_state",
  SetState: "set_state",
  GetStates: "get_states",
  MediaCommand: "media_command",
  GetMonitors: "get_monitors",
  SetTaskbarVisible: "set_taskbar_visible",
  SetApiKey: "set_api_key",
  GetNotifications: "get_notifications",
  PostNotification: "post_notification",
  DismissNotification: "dismiss_notification",
  ClearNotifications: "clear_notifications",
  GetVolume: "get_volume",
  SetVolume: "set_volume",
  SetMuted: "set_muted",
  StepVolume: "step_volume",
  GetBrightness: "get_brightness",
  SetBrightness: "set_brightness",
  StepBrightness: "step_brightness",
  SetNightLight: "set_night_light",
  GetMic: "get_mic",
  SetMic: "set_mic",
  SetMicMuted: "set_mic_muted",
  GetAudioSessions: "get_audio_sessions",
  SetSessionVolume: "set_session_volume",
  SetSessionMuted: "set_session_muted",
  GetRadios: "get_radios",
  SetRadio: "set_radio",
  ScanWifi: "scan_wifi",
  ConnectWifi: "connect_wifi",
  DisconnectWifi: "disconnect_wifi",
  GetBluetoothDevices: "get_bluetooth_devices",
  GetIdleInhibit: "get_idle_inhibit",
  SetIdleInhibit: "set_idle_inhibit",
  GetSystemInfo: "get_system_info",
  GetTodos: "get_todos",
  AddTodo: "add_todo",
  SetTodoDone: "set_todo_done",
  RemoveTodo: "remove_todo",
  ClearDoneTodos: "clear_done_todos",
  ReorderTodo: "reorder_todo",
  GetPersistent: "get_persistent",
  SetPersistentValue: "set_persistent_value",
} as const;

/** IPC targets, mirroring end4-pC's `IpcHandler` names. */
export const IpcTarget = {
  Background: "background",
  Bar: "bar",
  Config: "config",
  MediaControls: "mediaControls",
  SidebarLeft: "sidebarLeft",
  SidebarRight: "sidebarRight",
  Settings: "settings",
  Wallpapers: "wallpapers",
  WallpaperSelector: "wallpaperSelector",
} as const;

export interface WallpaperChanged {
  monitor: string;
  path: string;
  /** Set when work-safety blanking replaced the image with a flat colour. */
  blanked: boolean;
}

export interface ResourceReading {
  cpu: number;
  memory: number;
  memoryUsedBytes: number;
  memoryTotalBytes: number;
  swap: number;
  disk: number;
  diskUsedBytes: number;
  diskTotalBytes: number;
}

export interface MediaState {
  playing: boolean;
  title: string;
  artist: string;
  album: string;
  /** Seconds; `0` when the source does not report a position. */
  position: number;
  duration: number;
  /** Data URL for the album art, or an empty string. */
  artwork: string;
  /** The app that owns the session, e.g. `Spotify.exe`. */
  source: string;
}

export type MediaAction = "playPause" | "next" | "previous";

export interface BatteryState {
  present: boolean;
  percent: number;
  charging: boolean;
  /** Seconds remaining, or `null` when Windows cannot estimate it. */
  secondsRemaining: number | null;
}

export interface WeatherState {
  city: string;
  description: string;
  /** Celsius unless the config asks for USC units. */
  temperature: number;
  humidity: number;
  windSpeed: number;
  /** Material Symbols icon name. */
  icon: string;
  sunrise: string;
  sunset: string;
}

export interface Workspace {
  name: string;
  displayName: string;
  focused: boolean;
  populated: boolean;
  monitor: string;
}

export interface WorkspaceState {
  /** `"glazewm"`, `"komorebi"`, or `null` when no window manager was found. */
  source: string | null;
  workspaces: Workspace[];
}

export interface ActiveWindow {
  title: string;
  /** The window class, the closest Windows has to an application id. */
  class: string;
}

export interface NetworkReading {
  /** Bytes per second since the previous sample. */
  down: number;
  up: number;
  totalReceived: number;
  totalSent: number;
}

export interface TrayIcon {
  /** The owning window handle, as a hex string. */
  window: string;
  id: number;
  tooltip: string;
  /** Whether Explorer keeps this icon in the overflow flyout. */
  hidden: boolean;
}

export interface VolumeReading {
  /** 0–100. */
  percent: number;
  muted: boolean;
}

export interface RadiosState {
  /** `null` when the machine has no radio of that kind — the signal to hide
   * the tile rather than draw it greyed. */
  wifi: boolean | null;
  bluetooth: boolean | null;
  /** Radio access can be denied by the user or by policy. */
  canControl: boolean;
}

export interface WifiNetwork {
  ssid: string;
  /** 0–5, as Windows reports it. */
  bars: number;
  secured: boolean;
}

export type ConnectOutcome = "connected" | "badPassword" | "failed";

export interface BluetoothDeviceInfo {
  id: string;
  name: string;
  connected: boolean;
}

/** What the sidebar banner shows about the machine. */
export interface SystemInfo {
  username: string;
  hostname: string;
  /** Already formatted by the backend, so every surface words it identically. */
  uptime: string;
}

/** One application in the volume mixer. */
export interface AudioSession {
  /** Stable while the session lives, and not reused after it ends — unlike
   * the process id. */
  id: string;
  processId: number;
  name: string;
  /** A cached PNG path, or empty. Resolve with `backend().assetUrl`. */
  icon: string;
  /** 0–100. */
  percent: number;
  muted: boolean;
  /** Sessions that have stopped playing are still listed; the mixer dims them
   * rather than removing them, so a slider does not vanish under the pointer. */
  active: boolean;
}

export interface BrightnessReading {
  /** 0–100, or `null` when no display can report a level. */
  percent: number | null;
  /** Whether to draw the control at all. */
  supported: boolean;
}

/** What the backend asks the readout to show. */
export interface OsdReading {
  kind: "volume" | "brightness";
  value: number;
  muted: boolean;
}

export interface MonitorInfo {
  name: string;
  x: number;
  y: number;
  width: number;
  height: number;
  workWidth: number;
  workHeight: number;
  primary: boolean;
}
