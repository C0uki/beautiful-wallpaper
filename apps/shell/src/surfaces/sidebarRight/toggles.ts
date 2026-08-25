// What the quick toggles can do.
//
// One registry, two renderers: the classic row and the Android-style grid draw
// the same set differently. Keeping the definitions here is what stops the two
// styles from drifting into offering different things.

import { tr } from "../../i18n";
import { actions, useShell, type ShellState } from "../../shell/store";

export interface ToggleDefinition {
  id: string;
  /** Looked up when rendering, so it follows the UI language. */
  label: () => string;
  icon: string;
  /** The icon shown when it is on, when that differs. */
  iconOn?: string;
  /** `null` hides the tile entirely: the machine has no such hardware. */
  state: (shell: ShellState) => boolean | null;
  toggle: (shell: ShellState) => void;
  /** A secondary action — long press, or the chevron on an Android tile. */
  detail?: DetailDialog;
  /** Extra line on the Android tile. */
  detailText?: (shell: ShellState) => string | undefined;
}

export type DetailDialog = "wifi" | "bluetooth" | "mixer" | "nightLight";

export const TOGGLES: ToggleDefinition[] = [
  {
    id: "wifi",
    label: () => tr("Wi-Fi"),
    icon: "wifi_off",
    iconOn: "wifi",
    state: (shell) => shell.radios.wifi,
    toggle: (shell) => void actions.setRadio("wifi", !shell.radios.wifi),
    detail: "wifi",
  },
  {
    id: "bluetooth",
    label: () => tr("Bluetooth"),
    icon: "bluetooth_disabled",
    iconOn: "bluetooth",
    state: (shell) => shell.radios.bluetooth,
    toggle: (shell) =>
      void actions.setRadio("bluetooth", !shell.radios.bluetooth),
    detail: "bluetooth",
  },
  {
    id: "darkMode",
    label: () => tr("Dark mode"),
    icon: "light_mode",
    iconOn: "dark_mode",
    state: (shell) => shell.theme?.mode === "dark",
    toggle: (shell) =>
      void actions.setMode(shell.theme?.mode === "dark" ? "light" : "dark"),
  },
  {
    id: "doNotDisturb",
    label: () => tr("Do not disturb"),
    icon: "notifications",
    iconOn: "notifications_paused",
    state: (shell) => shell.config.notifications.doNotDisturb,
    toggle: (shell) =>
      void actions.setConfigValue(
        "notifications.doNotDisturb",
        !shell.config.notifications.doNotDisturb,
      ),
  },
  {
    id: "nightLight",
    label: () => tr("Night light"),
    icon: "bedtime_off",
    iconOn: "bedtime",
    state: (shell) => shell.config.sidebar.nightLight.enable,
    toggle: (shell) =>
      void actions.setNightLight(!shell.config.sidebar.nightLight.enable),
    detail: "nightLight",
    detailText: (shell) => `${shell.config.sidebar.nightLight.temperature}K`,
  },
  {
    id: "idleInhibit",
    label: () => tr("Keep awake"),
    icon: "bedtime",
    iconOn: "coffee",
    state: (shell) => shell.persistent.idle.inhibit,
    toggle: (shell) => {
      const next = !shell.persistent.idle.inhibit;
      void actions.setIdleInhibit(next);
      void actions.setPersistentValue("idle.inhibit", next);
    },
  },
  {
    id: "mic",
    label: () => tr("Microphone"),
    icon: "mic_off",
    iconOn: "mic",
    state: (shell) => !shell.mic.muted,
    toggle: (shell) => void actions.setMicMuted(!shell.mic.muted),
  },
  {
    id: "mixer",
    label: () => tr("Volume mixer"),
    icon: "tune",
    // Not a toggle at all: it only opens its dialog. Rendered as an always-off
    // tile so the grid does not need a second kind of cell.
    state: () => false,
    toggle: () => {},
    detail: "mixer",
    detailText: (shell) =>
      shell.sessions.length > 0
        ? tr("%1 apps").replace("%1", String(shell.sessions.length))
        : undefined,
  },
];

/** The toggles this machine can actually offer, in the user's order. */
export function useVisibleToggles(): ToggleDefinition[] {
  const shell = useShell();
  const layout = shell.persistent.sidebar.quickToggles;

  const available = TOGGLES.filter((toggle) => toggle.state(shell) !== null);
  if (layout.length === 0) return available;

  // A saved layout orders and filters; anything it has never heard of is
  // appended, so a toggle added in a later version is not invisible to
  // everybody who has ever opened the editor.
  const ordered = layout
    .filter((slot) => slot.enabled)
    .map((slot) => available.find((toggle) => toggle.id === slot.id))
    .filter((toggle): toggle is ToggleDefinition => Boolean(toggle));

  const known = new Set(layout.map((slot) => slot.id));
  return [...ordered, ...available.filter((toggle) => !known.has(toggle.id))];
}
