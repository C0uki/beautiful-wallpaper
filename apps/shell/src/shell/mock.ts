// A stand-in for the Rust backend.
//
// It serves plausible data for every command and pushes the same events on a
// timer, so the surfaces can be built, reviewed and screenshotted without
// Windows. Every value here is obviously synthetic — no real system is read.

import {
  Command,
  Event,
  defaultStates,
  isStateFlag,
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
  type AppEntry,
  type AppKind,
  type LauncherResult,
  type CaptureMode,
  type CaptureOutcome,
  type SessionAction,
  type MenuItem,
  type Placement,
  type ShelfItem,
  type PresetSummary,
  type Comparison,
  type KeyStatus,
  type DropOutcome,
  type ScreenChrome,
  type HotCorner,
  type Edge,
  type OverlayLayout,
  type OverlayWidget,
  type Placed,
  type Crosshair,
  defaultConfig,
} from "@bw/core";
import type { Backend } from "./backend";
import { gradientWallpaper, mockScreen } from "./mockWallpaper";

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

  /** The shape of `bw-core::chrome::ScreenChrome::resolve`.
   *
   * The rules that matter are in Rust under tests; this mirrors them closely
   * enough for the harness to draw something, and the harness is never
   * full-screen. */
  const screenChrome = (): ScreenChrome => {
    const bar = config.bar;
    const hugging = bar.enable && bar.style === "hug";
    const centreOnly = !bar.left.length && !bar.right.length;
    const barEdge: Edge = bar.vertical
      ? bar.bottom
        ? "right"
        : "left"
      : bar.bottom
        ? "bottom"
        : "top";

    return {
      cornersVisible: config.appearance.fakeScreenRounding !== 0,
      radius: config.appearance.screenRounding,
      frameEdges: bar.showFrame
        ? (["top", "bottom", "left", "right"] as Edge[]).filter(
            (edge) => !hugging || edge !== barEdge || centreOnly,
          )
        : [],
      frameThickness: bar.frameThickness,
      frameColor: bar.frameColor,
      hotCornersActive: config.sidebar.cornerOpen.enable,
    };
  };

  /** The shape of `bw-core::keys::report`.
   *
   * The table of what Windows keeps, the chord normalisation and the
   * suggestion ladder are in Rust under tests — including the one that holds
   * the shipped defaults against the table. This knows just enough of it for
   * the step to have something to show, plus a refusal, which is the part no
   * table can predict and no browser can produce. */
  const keyReport = (): KeyStatus[] => {
    const WINDOWS_KEEPS: Record<string, string> = {
      "shift+super+s": "the Snipping Tool",
      "shift+super+m": "restoring minimised windows",
      "super+i": "Settings",
    };
    const REFUSED = ["captureOcr"];

    const canonical = (chord: string) =>
      chord
        .toLowerCase()
        .split("+")
        .map((part) => (part === "win" || part === "meta" ? "super" : part))
        .sort((left, right) => left.localeCompare(right))
        .join("+");

    const entries = Object.entries(config.keybinds).filter(
      (entry): entry is [string, string] => typeof entry[1] === "string",
    );

    return entries.map(([binding, chord]) => {
      const sharedWith = entries
        .filter(
          ([other, otherChord]) =>
            other !== binding && canonical(otherChord) === canonical(chord),
        )
        .map(([other]) => other);
      const takenByWindows = WINDOWS_KEEPS[canonical(chord)] ?? null;
      const refused = REFUSED.includes(binding);
      const trouble =
        refused || takenByWindows !== null || sharedWith.length > 0;

      return {
        binding,
        chord,
        takenByWindows,
        sharedWith,
        refused,
        suggestion: trouble ? `Ctrl+Alt+${chord.split("+").pop()}` : null,
      };
    });
  };

  // Two presets to look at without having to save one first, and a third that
  // will not parse — the case where a preset is still on disk and the screen
  // has to say so rather than quietly dropping it.
  const presetConfig = (change: (draft: Config) => void): Config => {
    const draft = structuredClone(config);
    change(draft);
    return draft;
  };
  let presets: Array<{ summary: PresetSummary; config: Config }> = [
    {
      summary: {
        name: "Midnight",
        description: "dark, floating bar, no frame",
        created: "2026-08-11T22:14:00+09:00",
        wallpaper: `C:/Users/you/Pictures/Wallpapers/${SAMPLE_WALLPAPERS[1]!.name}`,
        problem: null,
      },
      config: presetConfig((draft) => {
        draft.appearance.roundingScale = 1.4;
        draft.appearance.screenRounding = 32;
        draft.appearance.transparency.extra = 0.15;
        draft.bar.style = "float";
        draft.bar.height = 44;
        draft.bar.showFrame = true;
        draft.bar.left = ["media", "clock"];
        draft.dock.enable = true;
        draft.sidebar.width = 0.3;
        draft.notifications.timeout = 4000;
        draft.overlay.clickthroughOpacity = 0.6;
        draft.background.wallpaperPath = `C:/Users/you/Pictures/Wallpapers/${SAMPLE_WALLPAPERS[1]!.name}`;
      }),
    },
    {
      summary: {
        name: "Daylight",
        description: "light, bar at the bottom",
        created: "2026-07-03T09:02:00+09:00",
        wallpaper: `C:/Users/you/Pictures/Wallpapers/${SAMPLE_WALLPAPERS[2]!.name}`,
        problem: null,
      },
      config: presetConfig((draft) => {
        draft.appearance.palette.mode = "light";
        draft.bar.bottom = true;
        draft.bar.height = 36;
        draft.dock.enable = true;
        draft.background.wallpaperPath = `C:/Users/you/Pictures/Wallpapers/${SAMPLE_WALLPAPERS[2]!.name}`;
      }),
    },
    {
      summary: {
        name: "Bent",
        description: "",
        created: "",
        wallpaper: "",
        problem:
          "C:\\Users\\you\\AppData\\Roaming\\beautiful-wallpaper\\presets\\Bent.json is not a preset: expected value at line 1 column 3",
      },
      config,
    },
  ];
  let presetUndo: Config | null = null;

  // A shelf with something on it, so the surface can be seen without a
  // Windows drag: one of each kind, and one whose file has gone — which is
  // the state that has to be visible rather than quietly tidied away.
  let shelfItems: ShelfItem[] = [
    {
      id: 3,
      path: "C:/Users/you/Downloads/quarterly-report.pdf",
      name: "quarterly-report.pdf",
      kind: "document",
      size: 2_411_008,
      missing: false,
    },
    {
      id: 2,
      path: "C:/Users/you/Pictures/Wallpapers/dunes-at-dusk.jpg",
      name: "dunes-at-dusk.jpg",
      kind: "image",
      size: 5_242_880,
      missing: false,
    },
    {
      id: 1,
      path: "C:/Users/you/Projects/beautiful-wallpaper",
      name: "beautiful-wallpaper",
      kind: "folder",
      size: null,
      missing: false,
    },
    {
      id: 0,
      path: "D:/Archive/holiday-2025.zip",
      name: "holiday-2025.zip",
      kind: "archive",
      size: 1_073_741_824,
      missing: true,
    },
  ];
  let nextShelfId = 4;

  /** Coarsely what `ShelfKind::of` does, for a harness that has no disk. */
  const shelfKind = (path: string): ShelfItem["kind"] => {
    const extension = path.split(".").pop()?.toLowerCase() ?? "";
    if (!path.includes(".")) return "folder";
    if (["png", "jpg", "jpeg", "gif", "webp"].includes(extension))
      return "image";
    if (["mp4", "mkv", "webm", "mov"].includes(extension)) return "video";
    if (["mp3", "flac", "wav", "ogg"].includes(extension)) return "audio";
    if (["pdf", "docx", "txt", "md", "csv"].includes(extension))
      return "document";
    if (["zip", "7z", "rar", "tar"].includes(extension)) return "archive";
    if (["ts", "tsx", "rs", "py", "json"].includes(extension)) return "code";
    return "other";
  };

  // The harness's stand-in for `GetCursorPos`: the shell reads the pointer at
  // the moment the menu is asked for, and in a browser the only way to know
  // where it is, is to have been watching.
  const pointer: Placement = { x: 0, y: 0 };
  if (typeof window !== "undefined") {
    window.addEventListener(
      "mousemove",
      (event) => {
        pointer.x = event.clientX;
        pointer.y = event.clientY;
      },
      { passive: true },
    );
  }

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
    // Nothing is ever full-screen in the harness, which is what makes the
    // corners and the frame visible in a screenshot.
    fullscreen: false,
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

  // What a launcher search runs over. Obviously synthetic, like everything
  // else here, and deliberately including a packaged entry so the row that
  // carries a different launch mechanism is exercised.
  const SAMPLE_APPS: AppEntry[] = [
    {
      name: "Visual Studio Code",
      target: "C:\\Programs\\Visual Studio Code.lnk",
      kind: "shortcut",
      icon: "",
      subtitle: "C:\\Program Files\\Microsoft VS Code\\Code.exe",
    },
    {
      name: "Calculator",
      target: "Microsoft.WindowsCalculator_8wekyb3d8bbwe!App",
      kind: "packaged",
      icon: "",
      subtitle: "Microsoft Store",
    },
    {
      name: "Windows Terminal",
      target: "Microsoft.WindowsTerminal_8wekyb3d8bbwe!App",
      kind: "packaged",
      icon: "",
      subtitle: "Microsoft Store",
    },
    {
      name: "Notepad",
      target: "C:\\Programs\\Notepad.lnk",
      kind: "shortcut",
      icon: "",
      subtitle: "C:\\Windows\\System32\\notepad.exe",
    },
    {
      name: "Control Panel",
      target: "C:\\Programs\\Control Panel.lnk",
      kind: "shortcut",
      icon: "",
      subtitle: "C:\\Windows\\System32\\control.exe",
    },
    {
      name: "Firefox",
      target: "C:\\Programs\\Firefox.lnk",
      kind: "shortcut",
      icon: "",
      subtitle: "C:\\Program Files\\Mozilla Firefox\\firefox.exe",
    },
  ];

  let pendingCapture: CaptureMode | null = null;

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
    // Done, so the first-run screen does not open itself over every other
    // screenshot. The harness opens it from its own button instead.
    firstRun: { done: true, step: 0 },
    overlay: {
      open: ["crosshair", "resources"],
      notesText: "",
      crosshair: {
        pinned: false,
        clickthrough: true,
        x: 928,
        y: 508,
        width: 0,
        height: 0,
      },
      notes: {
        pinned: false,
        clickthrough: false,
        x: 80,
        y: 120,
        width: 0,
        height: 0,
      },
      resources: {
        pinned: false,
        clickthrough: false,
        x: 80,
        y: 380,
        width: 0,
        height: 0,
      },
    },
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
          // The decorations follow the config rather than watching it, the
          // same way the real backend re-resolves and re-emits on a change.
          emit(Event.Chrome, screenChrome());
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
            if (name === "overlayOpen") {
              // The overlay's windows are not driven by the flag alone — the
              // real backend re-resolves the layout on every change, because
              // what is pinned survives the flag clearing.
              emit(Event.Overlay, await this.invoke(Command.GetOverlayLayout));
            }
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
          if (String(args["path"] ?? "").startsWith("overlay.")) {
            emit(Event.Overlay, await this.invoke(Command.GetOverlayLayout));
          }
          return persistent as T;
        }

        case Command.GetSessionActions: {
          // A plausible laptop: modern standby, and hibernation switched off
          // — which is the usual state of a machine bought in the last few
          // years, and the case that decides whether "sleep" is offered.
          const offered: SessionAction[] = [
            "lock",
            "sleep",
            "logOut",
            "restart",
            "shutDown",
          ];
          const session = config.session;
          return offered.filter((action) => {
            if (action === "lock") return session.lock;
            if (action === "sleep") return session.sleep;
            if (action === "logOut") return session.logOut;
            if (action === "restart") return session.restart;
            return session.shutDown;
          }) as T;
        }

        case Command.GetScreenChrome:
          return screenChrome() as T;

        case Command.GetHotCorners: {
          const corner = config.sidebar.cornerOpen;
          if (!corner.enable) return [] as HotCorner[] as T;
          const screen = {
            width: window.innerWidth,
            height: window.innerHeight,
          };
          const width = Math.min(corner.cornerRegionWidth, screen.width >> 1);
          const height = Math.min(
            corner.cornerRegionHeight,
            screen.height >> 1,
          );

          const made: HotCorner[] = [
            {
              corner: "topLeft" as const,
              x: 0,
              y: 0,
              action: corner.topLeftAction,
            },
            {
              corner: "topRight" as const,
              x: screen.width - width,
              y: 0,
              action: corner.topRightAction,
            },
            ...(corner.bottom
              ? [
                  {
                    corner: "bottomLeft" as const,
                    x: 0,
                    y: screen.height - height,
                    action: corner.bottomLeftAction,
                  },
                  {
                    corner: "bottomRight" as const,
                    x: screen.width - width,
                    y: screen.height - height,
                    action: corner.bottomRightAction,
                  },
                ]
              : []),
          ]
            .filter((made) => made.action.trim().length > 0)
            .map((made) => ({
              corner: made.corner,
              rect: { x: made.x, y: made.y, width, height },
              action: made.action,
            }));
          return made as T;
        }

        case Command.RunHotCorner: {
          const corner = String(args["corner"] ?? "");
          const open = config.sidebar.cornerOpen;
          const flag =
            corner === "topLeft"
              ? open.topLeftAction
              : corner === "topRight"
                ? open.topRightAction
                : corner === "bottomLeft"
                  ? open.bottomLeftAction
                  : open.bottomRightAction;
          if (isStateFlag(flag)) {
            states[flag] = !states[flag];
            emit(Event.StateChanged, states);
          }
          return undefined as T;
        }

        case Command.ScrollHotCorner:
          // Brightness and volume are both faked here already; a corner
          // scroll is the same call by another route.
          return undefined as T;

        // The rules that matter are in `bw-core::overlay` under tests; this
        // mirrors their shape so the harness can draw the canvas.
        case Command.GetOverlayLayout: {
          const sizes: Record<OverlayWidget, [number, number]> = {
            crosshair: [64, 64],
            notes: [280, 220],
            resources: [320, 210],
          };
          const overlay = persistent.overlay;
          const placed: Placed[] = overlay.open
            .filter((keyword, index) => overlay.open.indexOf(keyword) === index)
            .filter((keyword): keyword is OverlayWidget => keyword in sizes)
            .map((widget) => {
              const state = overlay[widget];
              const [width, height] = sizes[widget];
              return {
                widget,
                rect: {
                  x: state.x,
                  y: state.y,
                  width: state.width > 0 ? state.width : width,
                  height: state.height > 0 ? state.height : height,
                },
                pinned: state.pinned,
                clickthrough: state.clickthrough,
              };
            });

          if (!config.overlay.enable) {
            return {
              interactive: [],
              passive: [],
              region: [],
              interactiveVisible: false,
              passiveVisible: false,
              scrim: false,
            } satisfies OverlayLayout as T;
          }
          if (states.overlayOpen) {
            return {
              interactive: placed,
              passive: [],
              region: null,
              interactiveVisible: true,
              passiveVisible: false,
              scrim: config.overlay.darkenScreen,
            } satisfies OverlayLayout as T;
          }

          const pinned = placed.filter((found) => found.pinned);
          const passive = pinned.filter((found) => found.clickthrough);
          const interactive = pinned.filter((found) => !found.clickthrough);
          return {
            interactive,
            passive,
            region: interactive.map((found) => found.rect),
            interactiveVisible: interactive.length > 0,
            passiveVisible: passive.length > 0,
            scrim: false,
          } satisfies OverlayLayout as T;
        }

        // A plausible crosshair rather than a parse: the reader that matters
        // is `bw-core::crosshair`, and it has fifteen tests of its own.
        case Command.GetCrosshair:
          return {
            color: "#00FF00",
            outline: true,
            outlineOpacity: 0.5,
            outlineThickness: 1,
            centerDot: true,
            centerDotOpacity: 1,
            centerDotSize: 2,
            innerLines: true,
            innerLineOpacity: 0.8,
            innerLineLength: 10,
            innerLineVerticalLength: 10,
            innerLineThickness: 2,
            innerLineOffset: 4,
            outerLines: false,
            outerLineOpacity: 0.35,
            outerLineLength: 2,
            outerLineVerticalLength: 2,
            outerLineThickness: 2,
            outerLineOffset: 12,
            size: 32,
          } satisfies Crosshair as T;

        case Command.ToggleOverlayWidget: {
          const widget = String(args["widget"] ?? "");
          const open = persistent.overlay.open.includes(widget)
            ? persistent.overlay.open.filter((found) => found !== widget)
            : [...persistent.overlay.open, widget];
          persistent = {
            ...persistent,
            overlay: { ...persistent.overlay, open },
          };
          emit(Event.Persistent, persistent);
          // The layout follows the persisted state, so the canvas has to be
          // told — the real backend re-resolves and re-emits the same way.
          emit(Event.Overlay, await this.invoke(Command.GetOverlayLayout));
          return persistent as T;
        }

        case Command.GetShelfItems:
          return shelfItems as T;

        case Command.AddToShelf: {
          // The real rules are in `bw-core::shelf` under tests; this is the
          // shape of them, which is all the harness needs to draw a shelf.
          const incoming = (args["paths"] as string[] | undefined) ?? [];
          const outcome: DropOutcome = { added: 0, moved: 0, refused: 0 };
          const max = config.shelf.maxItems;

          for (const path of incoming) {
            const already = shelfItems.findIndex(
              (item) => item.path.toLowerCase() === path.toLowerCase(),
            );
            if (already >= 0) {
              const [existing] = shelfItems.splice(already, 1);
              shelfItems.unshift(existing!);
              outcome.moved += 1;
              continue;
            }
            if (shelfItems.length >= max) {
              outcome.refused += 1;
              continue;
            }
            shelfItems.unshift({
              id: nextShelfId++,
              path,
              name: path.split(/[/\\]/).pop() ?? path,
              kind: shelfKind(path),
              size: 1024 * 64,
              missing: false,
            });
            outcome.added += 1;
          }

          emit(Event.Shelf, shelfItems);
          return outcome as T;
        }

        case Command.RemoveFromShelf: {
          shelfItems = shelfItems.filter((item) => item.id !== args["id"]);
          emit(Event.Shelf, shelfItems);
          return shelfItems as T;
        }

        case Command.ClearShelf: {
          shelfItems = args["missingOnly"]
            ? shelfItems.filter((item) => !item.missing)
            : [];
          emit(Event.Shelf, shelfItems);
          return shelfItems as T;
        }

        case Command.OpenShelfItem:
        case Command.RevealShelfItem:
          // Nothing to open off Windows. Saying so beats pretending.
          throw new Error(
            "the mock backend cannot open files in a shell that is not running",
          );

        case Command.DragFromShelf:
          throw new Error(
            "the mock backend cannot hand files to Windows' drag and drop",
          );

        case Command.GetDesktopMenuItems: {
          const menu = config.desktopMenu;
          const offered: [MenuItem, boolean][] = [
            ["changeWallpaper", menu.changeWallpaper],
            ["nextWallpaper", menu.nextWallpaper],
            [
              "editWidgets",
              menu.editWidgets && config.background.widgets.enable,
            ],
            ["overview", menu.overview && config.overview.enable],
            ["screenshot", menu.screenshot && config.capture.enable],
            ["session", menu.session && config.session.enable],
            ["displaySettings", menu.displaySettings],
            ["personalise", menu.personalise],
          ];
          return offered
            .filter(([, shown]) => shown)
            .map(([item]) => item) as T;
        }

        // Deliberately the simple version. The rule that matters — flip to the
        // other side of the cursor rather than sliding back on screen — lives
        // in `bw-core::menu::place` under tests; the harness only needs the
        // menu to end up somewhere visible.
        case Command.PlaceDesktopMenu: {
          const width = Number(args["width"] ?? 0);
          const height = Number(args["height"] ?? 0);
          const margin = 8;
          const screen = {
            width: window.innerWidth,
            height: window.innerHeight,
          };
          const fit = (at: number, size: number, limit: number) => {
            if (at + size + margin <= limit) return at;
            if (at - size >= margin) return at - size;
            return Math.min(margin, Math.max(limit - size, 0));
          };
          return {
            x: fit(pointer.x, width, screen.width),
            y: fit(pointer.y, height, screen.height),
          } as T;
        }

        case Command.RunDesktopMenuItem: {
          states.desktopMenuOpen = false;
          const item = args["item"] as MenuItem;
          switch (item) {
            case "changeWallpaper":
              states.wallpaperSelectorOpen = true;
              break;
            case "editWidgets":
              states.widgetEditMode = !states.widgetEditMode;
              break;
            case "overview":
              states.overviewOpen = true;
              break;
            case "session":
              states.sessionOpen = true;
              break;
            default:
              // The rest need a machine — a wallpaper folder, a shutter, the
              // Settings app. Closing the menu is the whole visible effect.
              break;
          }
          emit(Event.StateChanged, states);
          return undefined as T;
        }

        case Command.ToggleDesktopMenu: {
          const action = String(args["action"] ?? "toggle");
          states.desktopMenuOpen =
            action === "open"
              ? true
              : action === "close"
                ? false
                : !states.desktopMenuOpen;
          emit(Event.StateChanged, states);
          return undefined as T;
        }

        case Command.RunSessionAction:
          // Nothing to end off Windows. Refusing rather than pretending keeps
          // the harness honest about which button was pressed.
          throw new Error(
            "the mock backend cannot end a session that is not running on Windows",
          );

        case Command.StartCapture: {
          pendingCapture = args["mode"] as CaptureMode;
          states.regionSelectOpen = true;
          emit(Event.StateChanged, states);
          emit(Event.Capture, {
            image: mockScreen(),
            width: 1920,
            height: 1080,
            mode: pendingCapture,
          });
          return undefined as T;
        }

        case Command.FinishCapture: {
          const mode = pendingCapture;
          if (mode === "screenshot") {
            states.regionSelectOpen = false;
            emit(Event.StateChanged, states);
            pendingCapture = null;
            return {
              saved:
                "C:\\Users\\you\\Pictures\\Screenshots\\Screenshot 2026-08-26 143012.png",
              text: null,
              translated: null,
              problem: null,
            } as T;
          }

          // Obviously synthetic, like everything else here: the "recognised"
          // text is the text drawn into the mock screen.
          const text =
            "Windows has a text recogniser built in. It only exists for languages whose pack is installed, so the shell asks before it offers to read anything.";
          const outcome: CaptureOutcome = {
            saved: null,
            text,
            translated:
              mode === "translate"
                ? "Windows にはテキスト認識機能が組み込まれています。言語パックが導入されている言語でのみ利用できるため、シェルは読み取りを提案する前に確認します。"
                : null,
            problem: null,
          };
          return outcome as T;
        }

        case Command.CancelCapture:
          pendingCapture = null;
          states.regionSelectOpen = false;
          emit(Event.StateChanged, states);
          return undefined as T;

        case Command.CanReadText:
          return true as T;

        case Command.GetLauncherResults:
          return mockLauncherResults(
            String(args["query"] ?? ""),
            dock,
            SAMPLE_APPS,
          ) as T;

        case Command.LaunchEntry:
        case Command.RunCommand:
          // Nothing to start off Windows. Returning rather than throwing keeps
          // the harness usable: the overview closes, as it would in the shell.
          return undefined as T;

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

        case Command.SearchBooru: {
          const page = Number(args["page"] ?? 1);
          // Synthetic, like everything else here: gradients rather than any
          // real image, and every result safe-rated so the mock exercises the
          // filtered path the shell ships with.
          return {
            page,
            items: Array.from({ length: 12 }, (_, index) => {
              const id = String(page * 100 + index);
              const palette: Array<[string, string]> = [
                ["#2a1330", "#b96a9a"],
                ["#16241f", "#7fa08a"],
                ["#101a2e", "#5f86c4"],
                ["#40260f", "#e3b579"],
              ];
              const [from, to] = palette[index % palette.length]!;
              const image = gradientWallpaper(from, to, id);
              return {
                id,
                width: 1920,
                height: index % 3 === 0 ? 2400 : 1080,
                preview: image,
                file: image,
                tags: ["scenery", "sky", "original"][index % 3]!,
                rating: "s",
                adult: false,
                pageUrl: `https://safebooru.org/index.php?page=post&s=view&id=${id}`,
              };
            }),
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

        case Command.GetPresets:
          return presets.map((entry) => entry.summary) as T;

        case Command.SavePreset: {
          const name = String(args["name"] ?? "").trim();
          // The real rules — device names, trailing dots, the characters a
          // file name cannot hold — are in `bw-core::preset` under tests. The
          // harness needs only the two the screen reacts to.
          if (!name) throw new Error("a preset needs a name");
          const taken = presets.find(
            (entry) => entry.summary.name.toLowerCase() === name.toLowerCase(),
          );
          if (taken && !args["overwrite"]) {
            throw new Error(`there is already a preset called \`${name}\``);
          }
          const summary: PresetSummary = {
            name: taken?.summary.name ?? name,
            description: String(args["description"] ?? "").trim(),
            created: new Date().toISOString(),
            wallpaper: config.background.wallpaperPath,
            problem: null,
          };
          presets = [
            ...presets.filter((entry) => entry !== taken),
            { summary, config: structuredClone(config) },
          ].sort((left, right) =>
            left.summary.name.localeCompare(right.summary.name),
          );
          return presets.map((entry) => entry.summary) as T;
        }

        case Command.RemovePreset: {
          const name = String(args["name"] ?? "");
          presets = presets.filter((entry) => entry.summary.name !== name);
          return presets.map((entry) => entry.summary) as T;
        }

        case Command.ComparePreset: {
          const found = presets.find(
            (entry) => entry.summary.name === String(args["name"] ?? ""),
          );
          if (!found) throw new Error("there is no preset called that");
          if (found.summary.problem) throw new Error(found.summary.problem);
          return comparePresets(config, found.config) as T;
        }

        case Command.ApplyPreset: {
          const found = presets.find(
            (entry) => entry.summary.name === String(args["name"] ?? ""),
          );
          if (!found) throw new Error("there is no preset called that");
          presetUndo = structuredClone(config);

          const next = structuredClone(config);
          for (const path of (args["paths"] as string[] | undefined) ?? []) {
            setByPath(
              next as unknown as Record<string, unknown>,
              path,
              readByPath(
                found.config as unknown as Record<string, unknown>,
                path,
              ),
            );
          }
          config = next;
          theme = retint(
            sampleTheme(
              config.appearance.palette.mode === "light" ? "light" : "dark",
            ),
            config.background.wallpaperPath,
          );
          emit(Event.ConfigChanged, config);
          emit(Event.ThemeChanged, theme);
          emit(Event.Chrome, screenChrome());
          return config as T;
        }

        case Command.HasPresetUndo:
          return (presetUndo !== null) as T;

        case Command.UndoPreset: {
          if (!presetUndo) throw new Error("there is nothing to undo");
          config = presetUndo;
          presetUndo = null;
          theme = retint(
            sampleTheme(
              config.appearance.palette.mode === "light" ? "light" : "dark",
            ),
            config.background.wallpaperPath,
          );
          emit(Event.ConfigChanged, config);
          emit(Event.ThemeChanged, theme);
          emit(Event.Chrome, screenChrome());
          return config as T;
        }

        case Command.GetKeyReport:
        case Command.RetryKeys:
          return keyReport() as T;

        case Command.DetectWindowManager:
          // Nothing is running in a browser, which is also the case worth
          // seeing: it is what the step has to explain.
          return null as T;

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
      // Already something a browser can load — the synthetic screen the region
      // picker draws is a data URL — so it is handed back untouched. Without
      // this every such image silently becomes a wallpaper gradient.
      if (/^(?:data|blob|https?):/.test(path)) return path;
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

function readByPath(root: Record<string, unknown>, path: string): unknown {
  let cursor: unknown = root;
  for (const segment of path.split(".")) {
    if (typeof cursor !== "object" || cursor === null) return undefined;
    cursor = (cursor as Record<string, unknown>)[segment];
  }
  return cursor;
}

/** The shape of `bw-core::preset::compare`.
 *
 * The rules that matter — which paths are safe to write, what happens to a key
 * this build has never heard of — are in Rust under tests. This walks the two
 * trees far enough for the confirm list to have something in it, with the one
 * rule that changes what the list looks like: an array is a leaf, so a
 * rearranged bar is one row rather than one per widget. */
function comparePresets(current: Config, preset: Config): Comparison {
  const shown = (value: unknown): string =>
    typeof value === "string"
      ? value
      : value === null || value === undefined
        ? ""
        : JSON.stringify(value);

  const leaves = (node: unknown, path: string): Array<[string, unknown]> =>
    node !== null && typeof node === "object" && !Array.isArray(node)
      ? Object.entries(node).flatMap(([key, child]) =>
          leaves(child, path ? `${path}.${key}` : key),
        )
      : path
        ? [[path, node]]
        : [];

  const mine = leaves(current, "");
  const theirs = new Map(leaves(preset, ""));

  return {
    changes: mine.flatMap(([path, value]) => {
      if (!theirs.has(path)) return [];
      const incoming = theirs.get(path);
      if (JSON.stringify(incoming) === JSON.stringify(value)) return [];
      return [{ path, from: shown(value), to: shown(incoming) }];
    }),
    unknown: [...theirs.keys()].filter(
      (path) => !mine.some(([mine]) => mine === path),
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

/** A stand-in for the launcher's result ordering.
 *
 * The real one lives in `bw-core` and is covered by tests there; this exists
 * so the surface can be built and screenshotted off Windows. It is
 * deliberately simpler — a greedy subsequence match rather than a search for
 * the best alignment, and arithmetic only between two numbers — so nothing
 * here should be read as the contract. */
function mockLauncherResults(
  query: string,
  dock: DockApp[],
  apps: AppEntry[],
): LauncherResult[] {
  const ACTIONS: Array<[string, string]> = [
    ["light", "light_mode"],
    ["dark", "dark_mode"],
    ["wallpaper", "wallpaper"],
    ["random", "shuffle"],
    ["widgets", "widgets"],
    ["sidebar", "right_panel_open"],
  ];

  const row = (
    kind: LauncherResult["kind"],
    title: string,
    subtitle: string,
    symbol: string,
    payload: string,
    positions: number[] = [],
    appKind: AppKind | null = null,
  ): LauncherResult => ({
    kind,
    title,
    subtitle,
    icon: "",
    symbol,
    payload,
    appKind,
    positions,
  });

  const trimmed = query.trim();

  if (trimmed.startsWith(">")) {
    const line = trimmed.slice(1).trim();
    return line ? [row("command", line, "", "terminal", line)] : [];
  }

  if (trimmed.startsWith("/")) {
    const rest = trimmed.slice(1).trim();
    return ACTIONS.flatMap(([keyword, symbol]) => {
      const positions = subsequence(keyword, rest);
      return positions
        ? [row("action", keyword, "", symbol, keyword, positions)]
        : [];
    });
  }

  const rows: LauncherResult[] = [];

  const arithmetic = /^\s*(-?[\d.]+)\s*([+\-*/%])\s*(-?[\d.]+)\s*$/.exec(
    trimmed,
  );
  if (arithmetic) {
    const left = Number(arithmetic[1]);
    const right = Number(arithmetic[3]);
    const answer = {
      "+": left + right,
      "-": left - right,
      "*": left * right,
      "/": left / right,
      "%": left % right,
    }[arithmetic[2]!];
    if (answer !== undefined && Number.isFinite(answer)) {
      const shown = String(answer);
      rows.push(row("calculator", shown, trimmed, "calculate", shown));
    }
  }

  const matched: LauncherResult[] = [];

  for (const app of dock) {
    for (const window of app.windows) {
      const positions = subsequence(window.title, trimmed);
      if (positions) {
        matched.push(
          row(
            "window",
            window.title,
            app.name,
            "select_window",
            window.id,
            positions,
          ),
        );
      }
    }
  }

  if (trimmed) {
    for (const app of apps) {
      const positions = subsequence(app.name, trimmed);
      if (positions) {
        matched.push(
          row(
            "app",
            app.name,
            app.subtitle,
            "apps",
            app.target,
            positions,
            app.kind,
          ),
        );
      }
    }
  }

  rows.push(...matched.slice(0, 8));

  if (trimmed) {
    rows.push(
      row(
        "webSearch",
        trimmed,
        "",
        "travel_explore",
        `https://www.google.com/search?q=${encodeURIComponent(trimmed)}`,
      ),
    );
  }

  return rows;
}

/** Greedy subsequence positions, or null when the query does not match. */
function subsequence(candidate: string, query: string): number[] | null {
  const needle = Array.from(query.toLowerCase()).filter(
    (character) => !/\s/.test(character),
  );
  if (!needle.length) return [];

  const haystack = Array.from(candidate.toLowerCase());
  const positions: number[] = [];
  let at = 0;
  for (const character of needle) {
    const found = haystack.indexOf(character, at);
    if (found < 0) return null;
    positions.push(found);
    at = found + 1;
  }
  return positions;
}
