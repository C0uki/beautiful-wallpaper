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
  type StateFlagName,
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
  /** The wallpaper currently applied, per monitor. */
  wallpaper: { path: string; blanked: boolean };
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
  wallpaper: { path: "", blanked: false },
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
  })();

  return connected;
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
};
