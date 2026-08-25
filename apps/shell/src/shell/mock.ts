// A stand-in for the Rust backend.
//
// It serves plausible data for every command and pushes the same events on a
// timer, so the surfaces can be built, reviewed and screenshotted without
// Windows. Every value here is obviously synthetic — no real system is read.

import {
  Command,
  Event,
  defaultStates,
  type BatteryState,
  type Config,
  type Entry,
  type EventName,
  type GeneratedTheme,
  type GlobalStates,
  type MediaState,
  type ResourceReading,
  type WallpaperPage,
  type WeatherState,
  type WorkspaceState,
  type ActiveWindow,
  type NetworkReading,
  type TrayIcon,
  type Notification,
  type OsdReading,
  type VolumeReading,
  type BrightnessReading,
  type AudioSession,
  type RadiosState,
  type WifiNetwork,
  type BluetoothDeviceInfo,
  type SystemInfo,
  type TodoItem,
  type Persistent,
  type DockApp,
  type ChatMessage,
  defaultConfig,
} from "@bw/core";
import type { Backend } from "./backend";
import { gradientWallpaper } from "./mockWallpaper";

type Handler = (payload: unknown) => void;

const SAMPLE_WALLPAPERS = [
  { name: "dunes-at-dusk.jpg", from: "#3b1f2b", to: "#c98b6b" },
  { name: "pine-fog.png", from: "#16241f", to: "#7fa08a" },
  { name: "harbour-lights.jpg", from: "#101a2e", to: "#5f86c4" },
  { name: "plum-hills.webp", from: "#2a1330", to: "#b96a9a" },
  { name: "desert-noon.jpg", from: "#40260f", to: "#e3b579" },
  { name: "glacier.png", from: "#0f2430", to: "#8fc7d8" },
  { name: "rooftops.jpg", from: "#2b2118", to: "#cf9f6b" },
  { name: "orchard.webp", from: "#1d2a13", to: "#a8c46a" },
];

const TERMINAL_DARK = [
  "#2b2224",
  "#f2708b",
  "#a8d18b",
  "#e8c07a",
  "#8fb8e0",
  "#d79ec4",
  "#8ed0c4",
  "#d5c2c6",
  "#5c4b4f",
  "#ff9db0",
  "#bfe0a6",
  "#f5d79a",
  "#a9cdf0",
  "#eab6da",
  "#a9e0d6",
  "#f5e8ea",
];

const TERMINAL_LIGHT = [
  "#f7eced",
  "#a3123e",
  "#3f6212",
  "#8a5a00",
  "#1f4d80",
  "#7b2f6a",
  "#0f6b5e",
  "#4a3b3e",
  "#8d7a7e",
  "#c4325c",
  "#557f22",
  "#a97400",
  "#356a9e",
  "#9a4a88",
  "#2a8a7a",
  "#241a1d",
];

const DARK_COLORS: Record<string, string> = {
  primary: "#ffb1c8",
  on_primary: "#5e1133",
  primary_container: "#7b2949",
  on_primary_container: "#ffd9e2",
  secondary: "#e3bdc6",
  on_secondary: "#422931",
  secondary_container: "#5a3f47",
  on_secondary_container: "#ffd9e2",
  tertiary: "#f0bd94",
  on_tertiary: "#48290c",
  tertiary_container: "#623f20",
  on_tertiary_container: "#ffdcc2",
  error: "#ffb4ab",
  on_error: "#690005",
  error_container: "#93000a",
  on_error_container: "#ffdad6",
  success: "#8fd88a",
  on_success: "#00390d",
  success_container: "#1f5223",
  on_success_container: "#aaf5a4",
  surface: "#191113",
  on_surface: "#efdfe1",
  on_surface_variant: "#d5c2c6",
  surface_container_lowest: "#130c0e",
  surface_container_low: "#211a1b",
  surface_container: "#261d20",
  surface_container_high: "#31282a",
  surface_container_highest: "#3c3235",
  surface_variant: "#524345",
  inverse_surface: "#efdfe1",
  inverse_on_surface: "#372e30",
  background: "#191113",
  on_background: "#efdfe1",
  outline: "#9e8c8f",
  outline_variant: "#524345",
  scrim: "#000000",
  shadow: "#000000",
  ...Object.fromEntries(
    TERMINAL_DARK.map((color, index) => [`term${index}`, color]),
  ),
};

const LIGHT_COLORS: Record<string, string> = {
  primary: "#8e4a63",
  on_primary: "#ffffff",
  primary_container: "#ffd9e2",
  on_primary_container: "#3b0721",
  secondary: "#74565e",
  on_secondary: "#ffffff",
  secondary_container: "#ffd9e2",
  on_secondary_container: "#2b151c",
  tertiary: "#7c5635",
  on_tertiary: "#ffffff",
  tertiary_container: "#ffdcc2",
  on_tertiary_container: "#2e1500",
  error: "#ba1a1a",
  on_error: "#ffffff",
  error_container: "#ffdad6",
  on_error_container: "#410002",
  success: "#2b6a2f",
  on_success: "#ffffff",
  success_container: "#aaf5a4",
  on_success_container: "#002204",
  surface: "#fff8f8",
  on_surface: "#22191c",
  on_surface_variant: "#524345",
  surface_container_lowest: "#ffffff",
  surface_container_low: "#fff0f2",
  surface_container: "#fbeaec",
  surface_container_high: "#f5e4e7",
  surface_container_highest: "#efdfe1",
  surface_variant: "#f3dde1",
  inverse_surface: "#382e30",
  inverse_on_surface: "#ffedef",
  background: "#fff8f8",
  on_background: "#22191c",
  outline: "#847375",
  outline_variant: "#d6c2c5",
  scrim: "#000000",
  shadow: "#000000",
  ...Object.fromEntries(
    TERMINAL_LIGHT.map((color, index) => [`term${index}`, color]),
  ),
};

function sampleTheme(mode: "light" | "dark"): GeneratedTheme {
  return {
    mode,
    variant: "tonalSpot",
    source: "#a8577a",
    wallpaperVibrancy: 0.42,
    colors: { ...(mode === "dark" ? DARK_COLORS : LIGHT_COLORS) },
  };
}

export function mockBackend(): Backend {
  const listeners = new Map<EventName, Set<Handler>>();
  let config: Config = structuredClone(defaultConfig);
  const states: GlobalStates = { ...defaultStates };
  let theme = sampleTheme("dark");

  const wallpapers: Entry[] = SAMPLE_WALLPAPERS.map(({ name }) => ({
    path: `C:/Users/you/Pictures/Wallpapers/${name}`,
    name,
    isDirectory: false,
  }));
  const folders: Entry[] = ["Anime", "Landscapes", "Minimal"].map((name) => ({
    path: `C:/Users/you/Pictures/Wallpapers/${name}`,
    name,
    isDirectory: true,
  }));

  const gradients = new Map(
    SAMPLE_WALLPAPERS.map(({ name, from, to }) => [
      `C:/Users/you/Pictures/Wallpapers/${name}`,
      gradientWallpaper(from, to, name),
    ]),
  );

  config.background.wallpaperPath = wallpapers[0]!.path;

  const emit = (event: EventName, payload: unknown) => {
    for (const handler of listeners.get(event) ?? []) handler(payload);
  };

  const resources = (): ResourceReading => {
    const wobble = (base: number, amplitude: number) =>
      Math.round(base + Math.sin(Date.now() / 4000 + base) * amplitude);
    return {
      cpu: wobble(21, 12),
      memory: wobble(48, 5),
      memoryUsedBytes: 15_300_000_000,
      memoryTotalBytes: 32_000_000_000,
      swap: 3,
      disk: 62,
      diskUsedBytes: 620_000_000_000,
      diskTotalBytes: 1_000_000_000_000,
    };
  };

  const media = (): MediaState => ({
    playing: true,
    title: "Feels Like the First Time",
    artist: "Foreigner",
    album: "Foreigner",
    position: 108 + ((Date.now() / 1000) % 60),
    duration: 233,
    artwork: "",
    source: "Spotify.exe",
  });

  const battery: BatteryState = {
    present: true,
    percent: 78,
    charging: false,
    secondsRemaining: 3 * 3600 + 20 * 60,
  };

  const weather: WeatherState = {
    city: "Kyoto",
    description: "overcast clouds",
    temperature: 21,
    humidity: 62,
    windSpeed: 2.2,
    icon: "cloud",
    sunrise: "05:12",
    sunset: "18:47",
  };

  const workspaces: WorkspaceState = {
    source: "glazewm",
    workspaces: [1, 2, 3, 4].map((index) => ({
      name: String(index),
      displayName: String(index),
      focused: index === 3,
      populated: index <= 3,
      monitor: "\\\\.\\DISPLAY1",
    })),
  };

  const activeWindow: ActiveWindow = {
    title: "beautiful-wallpaper — README.md",
    class: "Chrome_WidgetWin_1",
  };

  const network = (): NetworkReading => ({
    // A gentle wobble, so the bar's throughput widget has something to show.
    down: 240_000 + Math.sin(Date.now() / 3000) * 180_000,
    up: 42_000 + Math.cos(Date.now() / 2500) * 30_000,
    totalReceived: 18_400_000_000,
    totalSent: 2_100_000_000,
  });

  const tray: TrayIcon[] = [
    { window: "0x10a2c", id: 1, tooltip: "Sync client", hidden: false },
    { window: "0x2f110", id: 2, tooltip: "Audio mixer", hidden: false },
    { window: "0x3c884", id: 1, tooltip: "Update service", hidden: false },
    { window: "0x4a190", id: 7, tooltip: "Background task", hidden: true },
  ];

  let notifications: Notification[] = [
    {
      id: 3,
      appName: "beautiful-wallpaper",
      summary: "Wallpaper applied",
      body: "dunes-at-dusk.jpg — the palette follows it.",
      image: "",
      urgency: "normal",
      time: Math.floor(Date.now() / 1000) - 20,
      actions: [],
    },
    {
      id: 2,
      appName: "Sync client",
      summary: "3 files uploaded",
      body: "Everything in Pictures/Wallpapers is up to date.",
      image: "",
      urgency: "low",
      time: Math.floor(Date.now() / 1000) - 240,
      actions: [],
    },
    {
      id: 1,
      appName: "Update service",
      summary: "Restart required",
      body: "An update is waiting for the next restart.",
      image: "",
      urgency: "critical",
      time: Math.floor(Date.now() / 1000) - 3600,
      actions: [],
    },
  ];

  const volume: VolumeReading = { percent: 42, muted: false };
  // A machine that *can* report a level. The unsupported case is worth seeing
  // too, so it is reachable by setting this to null.
  let brightness: BrightnessReading = { percent: 65, supported: true };
  const mic: VolumeReading = { percent: 78, muted: false };

  let radios: RadiosState = { wifi: true, bluetooth: false, canControl: true };

  // A plausible dock: two pinned (one not running), several running, and one
  // application with two windows — the case the running-dots draw differently.
  const dockWindow = (exe: string, title: string, active = false) => ({
    id: `${exe}:${title}`,
    title,
    executable: exe,
    name: exe
      .split("\\")
      .pop()!
      .replace(/\.exe$/, ""),
    icon: "",
    active,
  });

  let dock: DockApp[] = [
    {
      executable: "c:\\apps\\editor.exe",
      name: "Editor",
      icon: "",
      windows: [],
      pinned: true,
      active: false,
    },
    {
      executable: "c:\\apps\\firefox.exe",
      name: "Firefox",
      icon: "",
      windows: [
        dockWindow("c:\\apps\\firefox.exe", "Inbox — Mail"),
        dockWindow("c:\\apps\\firefox.exe", "Docs", true),
      ],
      pinned: true,
      active: true,
    },
    {
      executable: "c:\\windows\\explorer.exe",
      name: "Explorer",
      icon: "",
      windows: [dockWindow("c:\\windows\\explorer.exe", "Downloads")],
      pinned: false,
      active: false,
    },
    {
      executable: "c:\\apps\\terminal.exe",
      name: "Terminal",
      icon: "",
      windows: [dockWindow("c:\\apps\\terminal.exe", "pwsh")],
      pinned: false,
      active: false,
    },
  ];

  // The mock has a key so the translator is exercised; flip to false to see
  // the first-run state instead.
  let hasAiKey = true;

  // A conversation that exercises what the window has to draw: a reply with
  // Markdown and a fenced code block, summarised thinking, and a web search
  // with sources.
  let chat: ChatMessage[] = [
    {
      id: 1,
      role: "user",
      content: "How do I reserve screen space for a bar on Windows?",
      thinking: "",
      searches: [],
      sources: [],
      attachments: [],
      answeredBy: "",
      time: Math.floor(Date.now() / 1000) - 300,
    },
    {
      id: 2,
      role: "assistant",
      content: [
        "You register an **app bar** with `SHAppBarMessage`. The sequence is:",
        "",
        "1. `ABM_NEW` to register the window",
        "2. `ABM_QUERYPOS` to ask where it may go",
        "3. `ABM_SETPOS` to commit, then move the window to the *granted* rect",
        "",
        "```rust",
        "let mut data = APPBARDATA {",
        "    cbSize: size_of::<APPBARDATA>() as u32,",
        "    hWnd: hwnd,",
        "    uEdge: ABE_TOP,",
        "    ..Default::default()",
        "};",
        "unsafe { SHAppBarMessage(ABM_NEW, &mut data) };",
        "```",
        "",
        "Windows may grant a different rectangle than the one you asked for, so",
        "always follow the grant rather than the request.",
      ].join("\n"),
      thinking:
        "The user is on Windows, so wlr-layer-shell does not apply. The Win32 equivalent is the app bar API.",
      searches: ["SHAppBarMessage reserve screen space"],
      sources: [
        {
          title: "SHAppBarMessage function — Win32 apps",
          url: "https://learn.microsoft.com/windows/win32/api/shellapi/nf-shellapi-shappbarmessage",
        },
      ],
      attachments: [],
      answeredBy: "",
      time: Math.floor(Date.now() / 1000) - 280,
    },
  ];
  let nextChatId = 3;

  // Signal bars deliberately span the range, including a 0-bar network — the
  // case an icon chosen by `bars > 0` gets wrong.
  const wifiNetworks: WifiNetwork[] = [
    { ssid: "Kingfisher", bars: 4, secured: true },
    { ssid: "Kingfisher-5G", bars: 3, secured: true },
    { ssid: "BT-Openreach", bars: 2, secured: true },
    { ssid: "Cafe Guest", bars: 1, secured: false },
    { ssid: "far-away-ap", bars: 0, secured: true },
  ];

  const bluetoothDevices: BluetoothDeviceInfo[] = [
    { id: "bt-1", name: "WH-1000XM4", connected: true },
    { id: "bt-2", name: "MX Master 3", connected: true },
    { id: "bt-3", name: "Kitchen Speaker", connected: false },
  ];

  const systemInfo: SystemInfo = {
    username: "you",
    hostname: "WORKSTATION",
    uptime: "2 days, 5 hours",
  };

  let idleInhibit = false;

  let todos: TodoItem[] = [
    { id: 1, content: "Reply to the shell review", done: false },
    { id: 2, content: "Write up the DDC/CI findings", done: false },
    { id: 3, content: "Ship the toast animation fix", done: true },
  ];
  let nextTodoId = 4;

  let persistent: Persistent = {
    sidebar: { bottomGroup: { tab: 0, collapsed: false }, quickToggles: [] },
    idle: { inhibit: false },
  };

  // A plausible mixer: something playing, something paused, and one entry with
  // no icon, because that is the case the layout most easily gets wrong.
  let sessions: AudioSession[] = [
    {
      id: "session-firefox",
      processId: 4821,
      name: "Firefox",
      icon: "",
      percent: 80,
      muted: false,
      active: true,
    },
    {
      id: "session-spotify",
      processId: 6120,
      name: "Spotify",
      icon: "",
      percent: 55,
      muted: false,
      active: true,
    },
    {
      id: "session-discord",
      processId: 3344,
      name: "Discord",
      icon: "",
      percent: 30,
      muted: true,
      active: false,
    },
  ];
  const osd: OsdReading = { kind: "volume", value: 42, muted: false };

  // Push the periodic events the real backend sends.
  const timers: ReturnType<typeof setInterval>[] = [];
  if (typeof window !== "undefined") {
    timers.push(setInterval(() => emit(Event.Resources, resources()), 2000));
    timers.push(setInterval(() => emit(Event.Network, network()), 2000));
    timers.push(setInterval(() => emit(Event.Media, media()), 1000));
  }

  const onlinePage = (page: number): WallpaperPage => ({
    page,
    totalPages: 4,
    items: Array.from({ length: 12 }, (_, index) => {
      const sample =
        SAMPLE_WALLPAPERS[(index + page) % SAMPLE_WALLPAPERS.length]!;
      const url = gradientWallpaper(
        sample.to,
        sample.from,
        `${sample.name}-${page}-${index}`,
      );
      return {
        id: `mock-${page}-${index}`,
        thumb: url,
        full: url,
        provider: "wallhaven" as const,
        title: sample.name.replace(/\.\w+$/, "").replace(/-/g, " "),
        author: "Mock Photographer",
        authorUrl: "https://example.invalid",
        likes: 40 + index * 7,
        width: 3840,
        height: 2160,
        downloadLocation: "",
      };
    }),
  });

  return {
    kind: "mock",

    async invoke<T>(
      command: string,
      args: Record<string, unknown> = {},
    ): Promise<T> {
      switch (command) {
        case Command.GetConfig:
          return config as T;
        case Command.GetTheme:
          return theme as T;
        case Command.GetStates:
          return states as T;

        case Command.SetConfigValue: {
          const path = String(args["path"] ?? "");
          // A fresh object every time, matching the real backend, which
          // serialises the config afresh. Mutating in place would leave every
          // nested reference identical, so a store selector returning an object
          // slice — `state.config.bar`, say — would never see the change.
          const next = structuredClone(config);
          setByPath(
            next as unknown as Record<string, unknown>,
            path,
            args["value"],
          );
          config = next;
          emit(Event.ConfigChanged, config);
          return config as T;
        }

        case Command.ToggleState:
        case Command.SetState: {
          const name = String(args["name"] ?? "") as keyof GlobalStates;
          if (name in states) {
            states[name] =
              command === Command.ToggleState
                ? !states[name]
                : Boolean(args["value"]);
            emit(Event.StateChanged, states);
          }
          return states as T;
        }

        case Command.ListWallpapers:
          return [...folders, ...wallpapers] as T;

        case Command.ThumbnailFor:
          return (gradients.get(String(args["path"])) ?? "") as T;

        case Command.ApplyWallpaper: {
          const path = String(args["path"] ?? "");
          config = structuredClone(config);
          config.background.wallpaperPath = path;
          // Re-tint the mock theme so applying a wallpaper visibly re-themes the
          // UI, the way the real generator does.
          theme = retint(sampleTheme(theme.mode), path);
          emit(Event.WallpaperChanged, {
            monitor: "\\\\.\\DISPLAY1",
            path,
            blanked: false,
          });
          emit(Event.ThemeChanged, theme);
          emit(Event.ConfigChanged, config);
          return undefined as T;
        }

        case Command.RandomWallpaper: {
          const others = wallpapers.filter(
            (w) => w.path !== config.background.wallpaperPath,
          );
          const pick = others[Math.floor(Math.random() * others.length)]!;
          return this.invoke<T>(Command.ApplyWallpaper, { path: pick.path });
        }

        case Command.SetMode: {
          const mode = args["mode"] === "light" ? "light" : "dark";
          theme = retint(sampleTheme(mode), config.background.wallpaperPath);
          config = structuredClone(config);
          config.appearance.palette.mode = mode;
          emit(Event.ThemeChanged, theme);
          return undefined as T;
        }

        case Command.SearchOnlineWallpapers:
          return onlinePage(Number(args["page"] ?? 1)) as T;

        case Command.DownloadWallpaper:
          return "C:/Users/you/Pictures/Wallpapers/downloaded.jpg" as T;

        case Command.MediaCommand:
        case Command.SetTaskbarVisible:
        case Command.SetApiKey:
        case Command.SetVolume:
        case Command.SetMuted:
        case Command.StepVolume:
          return undefined as T;

        case Command.GetNotifications:
          return notifications as T;

        case Command.GetVolume:
          return volume as T;

        case Command.GetBrightness:
          return brightness as T;

        case Command.GetMic:
          return mic as T;

        case Command.SetMic:
        case Command.SetMicMuted:
          return undefined as T;

        case Command.GetAudioSessions:
          return sessions as T;

        case Command.SetSessionVolume: {
          const id = String(args["id"]);
          const percent = Math.max(0, Math.min(100, Number(args["percent"])));
          // A fresh array and fresh entries, as everywhere else in the mock:
          // mutating in place would leave the store's selectors blind to it.
          sessions = sessions.map((session) =>
            session.id === id ? { ...session, percent } : session,
          );
          emit(Event.AudioSessions, sessions);
          return undefined as T;
        }

        case Command.SetSessionMuted: {
          const id = String(args["id"]);
          const muted = Boolean(args["muted"]);
          sessions = sessions.map((session) =>
            session.id === id ? { ...session, muted } : session,
          );
          emit(Event.AudioSessions, sessions);
          return undefined as T;
        }

        case Command.SetBrightness: {
          const percent = Math.max(0, Math.min(100, Number(args["percent"])));
          brightness = { ...brightness, percent };
          emit(Event.Brightness, percent);
          return undefined as T;
        }

        case Command.StepBrightness: {
          if (brightness.percent === null) return undefined as T;
          const step = args["up"] ? 5 : -5;
          const percent = Math.max(0, Math.min(100, brightness.percent + step));
          brightness = { ...brightness, percent };
          emit(Event.Brightness, percent);
          return undefined as T;
        }

        case Command.SetNightLight: {
          // The real backend persists the toggle through the config, so the
          // mock does too — and through a fresh object, as SetConfigValue does.
          const next = structuredClone(config);
          setByPath(
            next as unknown as Record<string, unknown>,
            "sidebar.nightLight.enable",
            Boolean(args["enable"]),
          );
          config = next;
          emit(Event.ConfigChanged, config);
          return config as T;
        }

        case Command.DismissNotification: {
          const id = Number(args["id"]);
          notifications = notifications.filter(
            (notification) => notification.id !== id,
          );
          emit(Event.Notifications, notifications);
          return undefined as T;
        }

        case Command.ClearNotifications: {
          notifications = [];
          emit(Event.Notifications, notifications);
          return undefined as T;
        }

        case Command.GetRadios:
          return radios as T;

        case Command.SetRadio: {
          const on = Boolean(args["on"]);
          radios =
            args["kind"] === "wifi"
              ? { ...radios, wifi: on }
              : { ...radios, bluetooth: on };
          return true as T;
        }

        case Command.ScanWifi:
          return wifiNetworks as T;

        case Command.ConnectWifi:
          // A wrong password is the one failure the dialog has to handle, so
          // the mock produces it for an obviously wrong one.
          return (
            args["password"] === "wrong" ? "badPassword" : "connected"
          ) as T;

        case Command.DisconnectWifi:
          return undefined as T;

        case Command.GetBluetoothDevices:
          return bluetoothDevices as T;

        case Command.GetSystemInfo:
          return systemInfo as T;

        case Command.GetIdleInhibit:
          return idleInhibit as T;

        case Command.SetIdleInhibit:
          idleInhibit = Boolean(args["on"]);
          return idleInhibit as T;

        case Command.GetTodos:
          return todos as T;

        case Command.AddTodo: {
          const content = String(args["content"] ?? "").trim();
          if (content) {
            todos = [...todos, { id: nextTodoId++, content, done: false }];
            emit(Event.Todos, todos);
          }
          return todos as T;
        }

        case Command.SetTodoDone: {
          const id = Number(args["id"]);
          const done = Boolean(args["done"]);
          todos = todos.map((todo) =>
            todo.id === id ? { ...todo, done } : todo,
          );
          emit(Event.Todos, todos);
          return todos as T;
        }

        case Command.RemoveTodo: {
          const id = Number(args["id"]);
          todos = todos.filter((todo) => todo.id !== id);
          emit(Event.Todos, todos);
          return todos as T;
        }

        case Command.ClearDoneTodos: {
          todos = todos.filter((todo) => !todo.done);
          emit(Event.Todos, todos);
          return todos as T;
        }

        case Command.ReorderTodo: {
          const id = Number(args["id"]);
          const to = Number(args["to"]);
          const from = todos.findIndex((todo) => todo.id === id);
          if (from >= 0 && to >= 0 && to < todos.length) {
            const next = [...todos];
            const [moved] = next.splice(from, 1);
            next.splice(to, 0, moved!);
            todos = next;
            emit(Event.Todos, todos);
          }
          return todos as T;
        }

        case Command.GetPersistent:
          return persistent as T;

        case Command.SetPersistentValue: {
          const next = structuredClone(persistent);
          setByPath(
            next as unknown as Record<string, unknown>,
            String(args["path"] ?? ""),
            args["value"],
          );
          persistent = next;
          emit(Event.Persistent, persistent);
          return persistent as T;
        }

        case Command.GetDockItems:
          return dock as T;

        case Command.ActivateWindow: {
          const id = String(args["id"]);
          const wasActive = dock.some((app) =>
            app.windows.some((w) => w.id === id && w.active),
          );
          if (wasActive && args["minimiseIfActive"]) return "minimised" as T;
          // Fresh objects throughout, as everywhere else in the mock.
          dock = dock.map((app) => ({
            ...app,
            active: app.windows.some((w) => w.id === id),
            windows: app.windows.map((w) => ({ ...w, active: w.id === id })),
          }));
          emit(Event.Dock, dock);
          return "activated" as T;
        }

        case Command.LaunchApp:
          return undefined as T;

        case Command.SetPinned: {
          const path = String(args["path"]).toLowerCase();
          const pinned = Boolean(args["pinned"]);
          dock = dock
            .map((app) =>
              app.executable.toLowerCase() === path ? { ...app, pinned } : app,
            )
            // Unpinning something that is not running takes it off the dock.
            .filter((app) => app.pinned || app.windows.length > 0);
          emit(Event.Dock, dock);
          return config as T;
        }

        case Command.HasAiKey:
          return hasAiKey as T;

        case Command.SetAiKey:
          hasAiKey = String(args["key"] ?? "").trim().length > 0;
          return undefined as T;

        case Command.Translate: {
          const text = String(args["text"] ?? "");
          if (!hasAiKey) return { text: "", error: "noKey" } as T;
          if (!text.trim()) return { text: "", error: null } as T;
          // Obviously synthetic, like every other value here — it reverses the
          // words so it is visibly not a real translation.
          return {
            text: text.split(/\s+/).reverse().join(" "),
            error: null,
          } as T;
        }

        case Command.PickFiles:
          // No real picker off Windows; a plausible path exercises the chip.
          return ["C:\\Users\\you\\Pictures\\diagram.png"] as T;

        case Command.GetChat:
          return chat as T;

        case Command.ClearChat:
          chat = [];
          emit(Event.Chat, chat);
          return chat as T;

        case Command.RetryChat:
          chat = chat.slice(0, -1);
          emit(Event.Chat, chat);
          return chat as T;

        case Command.SendChat: {
          const text = String(args["text"] ?? "");
          const attachments = (args["attachments"] as string[]) ?? [];
          if (!text.trim() && attachments.length === 0) return undefined as T;

          const blank = (): Omit<ChatMessage, "id" | "role" | "content"> => ({
            thinking: "",
            searches: [],
            sources: [],
            attachments: [],
            answeredBy: "",
            time: Math.floor(Date.now() / 1000),
          });

          chat = [
            ...chat,
            {
              id: nextChatId++,
              role: "user",
              content: text,
              ...blank(),
              attachments: attachments.map((path) =>
                String(path).split(/[\\/]/).pop()!,
              ),
            },
            { id: nextChatId++, role: "assistant", content: "", ...blank() },
          ];
          emit(Event.Chat, chat);

          // Streamed a word at a time, so the surface's incremental rendering
          // is exercised rather than a single finished reply appearing.
          const reply =
            "That depends on the surface. The bar reserves an edge; overlays do not.";
          const words = reply.split(" ");
          let index = 0;
          const tick = setInterval(() => {
            if (index >= words.length) {
              clearInterval(tick);
              emit(Event.ChatEvent, { kind: "done" });
              emit(Event.Chat, chat);
              return;
            }
            emit(Event.ChatEvent, {
              kind: "text",
              value: (index === 0 ? "" : " ") + words[index],
            });
            index += 1;
          }, 60);

          return undefined as T;
        }

        case Command.GetMonitors:
          return [
            {
              name: "\\\\.\\DISPLAY1",
              x: 0,
              y: 0,
              width: 1920,
              height: 1080,
              workWidth: 1920,
              workHeight: 1040,
              primary: true,
            },
          ] as T;

        default:
          throw new Error(`the mock backend has no command \`${command}\``);
      }
    },

    async listen<T>(event: EventName, handler: (payload: T) => void) {
      const set = listeners.get(event) ?? new Set<Handler>();
      set.add(handler as Handler);
      listeners.set(event, set);

      // Deliver current state immediately so a surface never renders empty.
      queueMicrotask(() => {
        if (event === Event.ConfigChanged) handler(config as T);
        if (event === Event.ThemeChanged) handler(theme as T);
        if (event === Event.StateChanged) handler(states as T);
        if (event === Event.Resources) handler(resources() as T);
        if (event === Event.Media) handler(media() as T);
        if (event === Event.Battery) handler(battery as T);
        if (event === Event.Weather) handler(weather as T);
        if (event === Event.Workspaces) handler(workspaces as T);
        if (event === Event.ActiveWindow) handler(activeWindow as T);
        if (event === Event.Network) handler(network() as T);
        if (event === Event.Tray) handler(tray as T);
        if (event === Event.Notifications) handler(notifications as T);
        if (event === Event.Volume) handler(volume as T);
        if (event === Event.Mic) handler(mic as T);
        if (event === Event.AudioSessions) handler(sessions as T);
        if (event === Event.Todos) handler(todos as T);
        if (event === Event.Persistent) handler(persistent as T);
        if (event === Event.Dock) handler(dock as T);
        if (event === Event.Chat) handler(chat as T);
        if (event === Event.Brightness) handler((brightness.percent ?? 0) as T);
        if (event === Event.Osd) handler(osd as T);
      });

      return () => {
        set.delete(handler as Handler);
      };
    },

    assetUrl(path: string) {
      return (
        gradients.get(path) ?? gradientWallpaper("#2a2126", "#7d5a68", path)
      );
    },
  };
}

/** Nudges the mock palette so each wallpaper looks like it re-themed the shell.
 *
 * The real backend re-runs Material Color Utilities over the new image. Here a
 * hue rotation of the whole palette is enough to show that every surface follows
 * the wallpaper — and rotating in HSL, rather than shifting raw channels, keeps
 * the tones plausible instead of turning containers garish.
 */
function retint(theme: GeneratedTheme, seed: string): GeneratedTheme {
  let hash = 0;
  for (const char of seed) hash = (hash * 31 + char.charCodeAt(0)) % 360;
  const shift = hash - 180;

  const rotate = (hex: string) => {
    if (!/^#[0-9a-f]{6}$/i.test(hex)) return hex;
    const [r, g, b] = [1, 3, 5].map(
      (at) => parseInt(hex.slice(at, at + 2), 16) / 255,
    ) as [number, number, number];
    const max = Math.max(r, g, b);
    const min = Math.min(r, g, b);
    const lightness = (max + min) / 2;
    if (max === min) return hex; // Grey has no hue to rotate.

    const delta = max - min;
    const saturation = delta / (1 - Math.abs(2 * lightness - 1));
    let hue =
      max === r
        ? ((g - b) / delta) % 6
        : max === g
          ? (b - r) / delta + 2
          : (r - g) / delta + 4;
    hue = (((hue * 60 + shift) % 360) + 360) % 360;

    const c = (1 - Math.abs(2 * lightness - 1)) * saturation;
    const x = c * (1 - Math.abs(((hue / 60) % 2) - 1));
    const m = lightness - c / 2;
    const sector = Math.floor(hue / 60) % 6;
    const rgb: [number, number, number] = [
      [c, x, 0],
      [x, c, 0],
      [0, c, x],
      [0, x, c],
      [x, 0, c],
      [c, 0, x],
    ][sector] as [number, number, number];

    return `#${rgb
      .map((channel) =>
        Math.round((channel + m) * 255)
          .toString(16)
          .padStart(2, "0"),
      )
      .join("")}`;
  };

  return {
    ...theme,
    source: rotate(theme.source),
    colors: Object.fromEntries(
      Object.entries(theme.colors).map(([key, value]) => [key, rotate(value)]),
    ),
  };
}

function setByPath(
  root: Record<string, unknown>,
  path: string,
  value: unknown,
): void {
  const segments = path.split(".");
  const leaf = segments.pop();
  if (!leaf) return;
  let cursor: Record<string, unknown> = root;
  for (const segment of segments) {
    const next = cursor[segment];
    if (typeof next !== "object" || next === null) return;
    cursor = next as Record<string, unknown>;
  }
  cursor[leaf] = value;
}
