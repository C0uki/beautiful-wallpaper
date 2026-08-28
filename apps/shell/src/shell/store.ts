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
  type AudioSession,
  type BrightnessReading,
  type Persistent,
  type RadiosState,
  type SystemInfo,
  type TodoItem,
  type VolumeReading,
  type WifiNetwork,
  type ConnectOutcome,
  type BluetoothDeviceInfo,
  type DockApp,
  type TranslationResult,
  type ActivateOutcome,
  type ChatMessage,
  type StreamEvent,
  type BooruPage,
  type LauncherResult,
  type AppKind,
  type CaptureMode,
  type CaptureOutcome,
  type Rect,
  type SessionAction,
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
  volume: VolumeReading;
  mic: VolumeReading;
  brightness: BrightnessReading;
  sessions: AudioSession[];
  radios: RadiosState;
  todos: TodoItem[];
  persistent: Persistent;
  systemInfo: SystemInfo | null;
  dock: DockApp[];
  /**
   * Bumped each time the application scan finishes. The overview watches it
   * rather than a list: it has to re-rank against what is typed anyway, so
   * carrying every application into every surface's store would be waste.
   */
  appsScanned: number;
  /** Whether an Anthropic key is configured; the translator needs one. */
  hasAiKey: boolean;
  chat: ChatMessage[];
  /** True while a reply is streaming in. */
  chatStreaming: boolean;
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
  volume: { percent: 0, muted: false },
  mic: { percent: 0, muted: false },
  // Assume unsupported until the backend says otherwise: showing a slider and
  // then removing it is worse than showing it a moment late.
  brightness: { percent: null, supported: false },
  sessions: [],
  radios: { wifi: null, bluetooth: null, canControl: false },
  todos: [],
  persistent: {
    sidebar: { bottomGroup: { tab: 0, collapsed: false }, quickToggles: [] },
    idle: { inhibit: false },
  },
  systemInfo: null,
  dock: [],
  appsScanned: 0,
  hasAiKey: false,
  chat: [],
  chatStreaming: false,
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
      api.listen<VolumeReading>(Event.Volume, (volume) => set({ volume })),
      api.listen<VolumeReading>(Event.Mic, (mic) => set({ mic })),
      // The backend only pushes a level when there is one to push, so an
      // arriving event is itself the evidence that the control exists.
      api.listen<number>(Event.Brightness, (percent) =>
        set({ brightness: { percent, supported: true } }),
      ),
      api.listen<AudioSession[]>(Event.AudioSessions, (sessions) =>
        set({ sessions }),
      ),
      api.listen<TodoItem[]>(Event.Todos, (todos) => set({ todos })),
      api.listen<Persistent>(Event.Persistent, (persistent) =>
        set({ persistent }),
      ),
      api.listen<DockApp[]>(Event.Dock, (dock) => set({ dock })),
      // The scan carries nothing; the overview re-runs its own query, which
      // is what it would have to do anyway to re-rank against what is typed.
      api.listen(Event.Apps, () =>
        set((state) => ({ appsScanned: state.appsScanned + 1 })),
      ),
      api.listen<ChatMessage[]>(Event.Chat, (chat) => set({ chat })),
      // Deltas arrive on their own channel and are applied to the last
      // message in place. Re-sending the whole conversation per token would
      // redraw every message in the window for each character.
      api.listen<StreamEvent>(Event.ChatEvent, (event) =>
        set((state) => applyStreamEvent(state, event)),
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

let sidebarConnected: Promise<void> | undefined;

/**
 * Fetches what only the sidebar needs.
 *
 * Kept out of `connect` deliberately: enumerating radios and audio sessions
 * costs real time, and the bar and the background surface would pay it at
 * startup for data they never draw.
 */
export function connectSidebar(): Promise<void> {
  sidebarConnected ??= (async () => {
    const api = backend();
    await connect();

    const [volume, mic, brightness, sessions, todos, persistent, systemInfo] =
      await Promise.all([
        api.invoke<VolumeReading>(Command.GetVolume),
        api.invoke<VolumeReading>(Command.GetMic),
        api.invoke<BrightnessReading>(Command.GetBrightness),
        api.invoke<AudioSession[]>(Command.GetAudioSessions),
        api.invoke<TodoItem[]>(Command.GetTodos),
        api.invoke<Persistent>(Command.GetPersistent),
        api.invoke<SystemInfo>(Command.GetSystemInfo),
      ]);

    set({ volume, mic, brightness, sessions, todos, persistent, systemInfo });

    // Radios come separately: a denied access request or a missing adapter
    // must not stop the rest of the sidebar from filling in.
    void api
      .invoke<RadiosState>(Command.GetRadios)
      .then((radios) => set({ radios }))
      .catch(() => {});
  })();

  return sidebarConnected;
}

let dockConnected: Promise<void> | undefined;

/** Fetches the dock's icons. Kept out of `connect` for the same reason the
 * sidebar's data is: enumerating every window costs time no other surface
 * should pay at startup. */
export function connectDock(): Promise<void> {
  dockConnected ??= (async () => {
    await connect();
    set({ dock: await backend().invoke<DockApp[]>(Command.GetDockItems) });
  })();
  return dockConnected;
}

/** Folds one streamed delta into the conversation. */
function applyStreamEvent(
  state: ShellState,
  event: StreamEvent,
): Partial<ShellState> {
  if (event.kind === "done" || event.kind === "failed") {
    return { chatStreaming: false };
  }

  const chat = [...state.chat];
  const index = chat.length - 1;
  const last = chat[index];
  // A delta with nothing to apply it to means the conversation has not caught
  // up yet; the `bw://chat` event that follows will carry the whole thing.
  if (!last || last.role !== "assistant") return { chatStreaming: true };

  const next = { ...last };
  switch (event.kind) {
    case "text":
      next.content += event.value;
      break;
    case "thinking":
      next.thinking += event.value;
      break;
    case "search":
      next.searches = [...next.searches, event.value];
      break;
    case "sources":
      next.sources = [...next.sources, ...event.value];
      break;
    case "fellBackTo":
      next.answeredBy = event.value;
      break;
  }

  chat[index] = next;
  return { chat, chatStreaming: true };
}

let leftConnected: Promise<void> | undefined;

/** What the left sidebar needs beyond the shared state. */
export function connectSidebarLeft(): Promise<void> {
  leftConnected ??= (async () => {
    const api = backend();
    await connect();
    const [hasAiKey, chat] = await Promise.all([
      api.invoke<boolean>(Command.HasAiKey),
      api.invoke<ChatMessage[]>(Command.GetChat),
    ]);
    set({ hasAiKey, chat });
  })();
  return leftConnected;
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
  sidebarConnected = undefined;
  dockConnected = undefined;
  leftConnected = undefined;
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
  setMic(percent: number) {
    return backend().invoke<void>(Command.SetMic, { percent });
  },
  setMicMuted(muted: boolean) {
    return backend().invoke<void>(Command.SetMicMuted, { muted });
  },
  setBrightness(percent: number) {
    return backend().invoke<void>(Command.SetBrightness, { percent });
  },
  setNightLight(enable: boolean) {
    return backend().invoke<Config>(Command.SetNightLight, { enable });
  },
  setSessionVolume(id: string, percent: number) {
    return backend().invoke<void>(Command.SetSessionVolume, { id, percent });
  },
  setSessionMuted(id: string, muted: boolean) {
    return backend().invoke<void>(Command.SetSessionMuted, { id, muted });
  },
  async setRadio(kind: "wifi" | "bluetooth", on: boolean) {
    await backend().invoke<boolean>(Command.SetRadio, { kind, on });
    // Re-read rather than assuming: the request can be refused, and a toggle
    // that shows the state the user asked for rather than the one they got is
    // worse than one that lags by a round trip.
    const radios = await backend().invoke<RadiosState>(Command.GetRadios);
    set({ radios });
  },
  scanWifi() {
    return backend().invoke<WifiNetwork[]>(Command.ScanWifi);
  },
  connectWifi(ssid: string, password?: string) {
    return backend().invoke<ConnectOutcome>(Command.ConnectWifi, {
      ssid,
      password: password ?? null,
    });
  },
  disconnectWifi() {
    return backend().invoke<void>(Command.DisconnectWifi);
  },
  bluetoothDevices() {
    return backend().invoke<BluetoothDeviceInfo[]>(Command.GetBluetoothDevices);
  },
  setIdleInhibit(on: boolean) {
    return backend().invoke<boolean>(Command.SetIdleInhibit, { on });
  },
  addTodo(content: string) {
    return backend().invoke<TodoItem[]>(Command.AddTodo, { content });
  },
  setTodoDone(id: number, done: boolean) {
    return backend().invoke<TodoItem[]>(Command.SetTodoDone, { id, done });
  },
  removeTodo(id: number) {
    return backend().invoke<TodoItem[]>(Command.RemoveTodo, { id });
  },
  clearDoneTodos() {
    return backend().invoke<TodoItem[]>(Command.ClearDoneTodos);
  },
  async refreshDock() {
    set({ dock: await backend().invoke<DockApp[]>(Command.GetDockItems) });
  },
  activateWindow(id: string, minimiseIfActive: boolean) {
    return backend().invoke<ActivateOutcome>(Command.ActivateWindow, {
      id,
      minimiseIfActive,
    });
  },
  launchApp(path: string) {
    return backend().invoke<void>(Command.LaunchApp, { path });
  },
  setPinned(path: string, pinned: boolean) {
    return backend().invoke<Config>(Command.SetPinned, { path, pinned });
  },
  translate(text: string, from: string, to: string) {
    return backend().invoke<TranslationResult>(Command.Translate, {
      text,
      from,
      to,
    });
  },
  async setAiKey(key: string) {
    await backend().invoke<void>(Command.SetAiKey, { key });
    set({ hasAiKey: await backend().invoke<boolean>(Command.HasAiKey) });
  },
  /** Saves an online image locally and returns its path. */
  downloadWallpaper(url: string, provider: string) {
    return backend().invoke<string>(Command.DownloadWallpaper, {
      url,
      provider,
      downloadLocation: null,
    });
  },
  searchBooru(tags: string, page: number) {
    return backend().invoke<BooruPage>(Command.SearchBooru, { tags, page });
  },
  launcherResults(query: string) {
    return backend().invoke<LauncherResult[]>(Command.GetLauncherResults, {
      query,
    });
  },
  launchEntry(target: string, kind: AppKind) {
    return backend().invoke<void>(Command.LaunchEntry, { target, kind });
  },
  runCommand(line: string) {
    return backend().invoke<void>(Command.RunCommand, { line });
  },
  startCapture(mode: CaptureMode) {
    return backend().invoke<void>(Command.StartCapture, { mode });
  },
  /** `scale` turns the overlay's CSS pixels into the frozen frame's own. */
  finishCapture(region: Rect, scale: number) {
    return backend().invoke<CaptureOutcome>(Command.FinishCapture, {
      region,
      scale,
    });
  },
  cancelCapture() {
    return backend().invoke<void>(Command.CancelCapture);
  },
  sessionActions() {
    return backend().invoke<SessionAction[]>(Command.GetSessionActions);
  },
  runSessionAction(action: SessionAction) {
    return backend().invoke<void>(Command.RunSessionAction, { action });
  },
  openUrl(url: string) {
    return backend().invoke<void>("plugin:opener|open_url", { url });
  },
  /** Opens a file picker and returns the chosen paths. */
  async pickFiles(): Promise<string[]> {
    const picked = await backend().invoke<string[] | null>(Command.PickFiles);
    return picked ?? [];
  },
  async sendChat(text: string, attachments: string[] = []) {
    set({ chatStreaming: true });
    try {
      await backend().invoke<void>(Command.SendChat, { text, attachments });
    } catch (error) {
      // The backend refuses a second send while one is in flight; the flag
      // has to come back down or the input stays disabled forever.
      set({ chatStreaming: false });
      throw error;
    }
  },
  async clearChat() {
    set({ chat: await backend().invoke<ChatMessage[]>(Command.ClearChat) });
  },
  async retryChat() {
    set({ chat: await backend().invoke<ChatMessage[]>(Command.RetryChat) });
  },
  setPersistentValue(path: string, value: unknown) {
    return backend().invoke<Persistent>(Command.SetPersistentValue, {
      path,
      value,
    });
  },
};
