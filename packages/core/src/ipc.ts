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
