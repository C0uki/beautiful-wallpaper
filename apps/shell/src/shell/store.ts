// One store per surface, mirroring the backend.
//
// end4-pC keeps this state in Quickshell singletons (`Config`, `Appearance`,
// `GlobalStates`) that every panel binds to. Each surface here is a separate
// webview, so instead each one subscribes to the same backend events and keeps
// its own copy — the backend stays the single writer.

import {
  Command,
  Event,
  defaultConfig,
  defaultStates,
  type BatteryState,
  type Config,
  type GeneratedTheme,
  type GlobalStates,
  type MediaState,
  type ResourceReading,
  type ActiveWindow,
  type Notification,
  type NetworkReading,
  type StateFlagName,
  type TrayIcon,
  type WeatherState,
  type WorkspaceState,
} from "@bw/core";
import { create } from "zustand";
import { backend } from "./backend";

export interface ShellState {
  ready: boolean;
  config: Config;
  theme: GeneratedTheme | null;
  states: GlobalStates;
  resources: ResourceReading | null;
  media: MediaState | null;
  battery: BatteryState | null;
  weather: WeatherState | null;
  workspaces: WorkspaceState | null;
  activeWindow: ActiveWindow | null;
  network: NetworkReading | null;
  tray: TrayIcon[];
  notifications: Notification[];
  /** The wallpaper currently applied, per monitor. */
  wallpaper: { path: string; blanked: boolean };
  /** Ticked once a second, so every clock in a surface stays in step. */
  now: Date;
}

const initial: ShellState = {
  ready: false,
  config: defaultConfig,
  theme: null,
  states: defaultStates,
  resources: null,
  media: null,
  battery: null,
  weather: null,
  workspaces: null,
  activeWindow: null,
  network: null,
  tray: [],
  notifications: [],
  wallpaper: { path: "", blanked: false },
  now: new Date(),
};

export const useShell = create<ShellState>(() => initial);

const set = useShell.setState;

let connected: Promise<void> | undefined;

/** Subscribes to the backend. Safe to call from several components. */
export function connect(): Promise<void> {
  connected ??= (async () => {
    const api = backend();

    await Promise.all([
      api.listen<Config>(Event.ConfigChanged, (config) => set({ config })),
      api.listen<GeneratedTheme>(Event.ThemeChanged, (theme) => set({ theme })),
      api.listen<GlobalStates>(Event.StateChanged, (states) => set({ states })),
      api.listen<ResourceReading>(Event.Resources, (resources) =>
        set({ resources }),
      ),
      api.listen<MediaState>(Event.Media, (media) => set({ media })),
      api.listen<BatteryState>(Event.Battery, (battery) => set({ battery })),
      api.listen<WeatherState>(Event.Weather, (weather) => set({ weather })),
      api.listen<WorkspaceState>(Event.Workspaces, (workspaces) =>
        set({ workspaces }),
      ),
      api.listen<ActiveWindow>(Event.ActiveWindow, (activeWindow) =>
        set({ activeWindow }),
      ),
      api.listen<NetworkReading>(Event.Network, (network) => set({ network })),
      api.listen<TrayIcon[]>(Event.Tray, (tray) => set({ tray })),
      api.listen<Notification[]>(Event.Notifications, (notifications) =>
        set({ notifications }),
      ),
      api.listen<{ path: string; blanked: boolean }>(
        Event.WallpaperChanged,
        (wallpaper) => set({ wallpaper }),
      ),
    ]);

    // Events cover changes; these fill in the state that already existed when the
    // surface opened.
    const [config, theme, states] = await Promise.all([
      api.invoke<Config>(Command.GetConfig),
      api.invoke<GeneratedTheme>(Command.GetTheme),
      api.invoke<GlobalStates>(Command.GetStates),
    ]);

    set({
      config,
      theme,
      states,
      ready: true,
      wallpaper: { path: config.background.wallpaperPath, blanked: false },
    });

    startClock();
  })();

  return connected;
}

/** Ticks `now` on the second boundary, so a minute never appears to change late. */
function startClock(): void {
  const schedule = () => {
    const delay = 1000 - (Date.now() % 1000);
    setTimeout(() => {
      set({ now: new Date() });
      schedule();
    }, delay);
  };
  schedule();
}

/** Resets the connection, for tests. */
export function resetShell(): void {
  connected = undefined;
  set(initial, true);
}

// Actions — thin wrappers so components never name a command string.

export const actions = {
  setConfigValue(path: string, value: unknown) {
    return backend().invoke<Config>(Command.SetConfigValue, { path, value });
  },
  applyWallpaper(path: string) {
    return backend().invoke<void>(Command.ApplyWallpaper, { path });
  },
  randomWallpaper() {
    return backend().invoke<void>(Command.RandomWallpaper);
  },
  setMode(mode: "light" | "dark") {
    return backend().invoke<void>(Command.SetMode, { mode });
  },
  toggleState(name: StateFlagName) {
    return backend().invoke<GlobalStates>(Command.ToggleState, { name });
  },
  setState(name: StateFlagName, value: boolean) {
    return backend().invoke<GlobalStates>(Command.SetState, { name, value });
  },
  mediaCommand(action: "playPause" | "next" | "previous") {
    return backend().invoke<void>(Command.MediaCommand, { action });
  },
  dismissNotification(id: number) {
    return backend().invoke<void>(Command.DismissNotification, { id });
  },
  clearNotifications() {
    return backend().invoke<void>(Command.ClearNotifications);
  },
  setVolume(percent: number) {
    return backend().invoke<void>(Command.SetVolume, { percent });
  },
  setMuted(muted: boolean) {
    return backend().invoke<void>(Command.SetMuted, { muted });
  },
};
