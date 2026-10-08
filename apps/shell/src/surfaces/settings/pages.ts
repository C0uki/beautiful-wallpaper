// Which settings appear on which settings page.
//
// The rows themselves are generated from the Rust schema, so nothing here has
// to list a key — only where each part of the config belongs and in what
// order. That is the one thing the schema genuinely cannot say.
//
// Pages are laid out the way the user looks for a setting, not the way the
// config happens to nest: night light lives in `sidebar` because the sidebar
// draws its toggle, but it is found under Displays. So a page claims config
// paths by prefix, and the longest claim wins — `sidebar.nightLight` takes
// those rows from `sidebar`.
//
// A setting with no page would be one nobody can reach, which is exactly the
// silent failure the generated form exists to avoid, so `pages.test.ts` holds
// every field in the schema against this table.

import type { Field } from "@bw/core";
import { tr } from "../../i18n";

export interface SettingsPage {
  id: string;
  icon: string;
  /** The tile behind the icon, as in the Mac's System Settings: a fixed hue
   *  per page, so a page is found by its colour as much as by its name. */
  tint: string;
  /** Pages are listed in groups, with a gap between each. */
  group: "basics" | "shell" | "features" | "other";
  title: () => string;
  /** One line under the title saying what is on the page. */
  summary: () => string;
  /** Config paths the page claims, as prefixes, in the order they appear. */
  paths: string[];
  /** Shown above the first row, when the page needs a warning. */
  caution?: () => string;
  /**
   * A page that is not a list of settings.
   *
   * Presets are the whole config under a name rather than one key each, so
   * they have no rows the schema could generate; the page draws itself. A
   * `custom` page claims no paths, which is why `pages.test.ts` still holds
   * for it.
   */
  custom?: "presets";
}

export const PAGES: SettingsPage[] = [
  {
    id: "general",
    icon: "settings",
    tint: "#8e8e93",
    group: "basics",
    title: () => tr("General"),
    summary: () => tr("Language, clock, weather, starting and updating"),
    paths: [
      "language",
      "time",
      "weather",
      "windows.startWithWindows",
      "windows.autoUpdate",
    ],
  },
  {
    id: "appearance",
    icon: "contrast",
    tint: "#0a84ff",
    group: "basics",
    title: () => tr("Appearance"),
    summary: () => tr("Colours, fonts, transparency and rounding"),
    paths: ["appearance", "windows.backdrop"],
  },
  {
    id: "wallpaper",
    icon: "photo_library",
    tint: "#32ade6",
    group: "basics",
    title: () => tr("Wallpaper"),
    summary: () => tr("The wallpaper, how it changes, and the picker"),
    paths: ["background", "wallpaperSelector"],
  },
  {
    id: "desktop",
    icon: "widgets",
    tint: "#af52de",
    group: "basics",
    title: () => tr("Desktop"),
    summary: () => tr("Widgets on the wallpaper, and the right-click menu"),
    paths: ["background.widgets", "desktopMenu"],
  },
  {
    id: "bar",
    icon: "toolbar",
    tint: "#5856d6",
    group: "shell",
    title: () => tr("Bar"),
    summary: () => tr("Where the bar sits, how it looks, and what it carries"),
    paths: ["bar"],
  },
  {
    id: "dock",
    icon: "dock_to_bottom",
    tint: "#5856d6",
    group: "shell",
    title: () => tr("Dock"),
    summary: () => tr("Pinned apps, size, hiding, and the Windows taskbar"),
    paths: ["dock", "windows.hideSystemTaskbar"],
  },
  {
    id: "sidebars",
    icon: "view_sidebar",
    tint: "#0a84ff",
    group: "shell",
    title: () => tr("Sidebars"),
    summary: () => tr("Both sidebars, quick toggles, and the screen corners"),
    paths: ["sidebar"],
  },
  {
    id: "notifications",
    icon: "notifications",
    tint: "#ff3b30",
    group: "shell",
    title: () => tr("Notifications"),
    summary: () => tr("Where toasts appear and how long they stay"),
    paths: ["notifications"],
  },
  {
    id: "displays",
    icon: "brightness_medium",
    tint: "#32ade6",
    group: "shell",
    title: () => tr("Displays"),
    summary: () => tr("Night light, and the volume and brightness readout"),
    paths: ["sidebar.nightLight", "osd"],
  },
  {
    id: "sound",
    icon: "volume_up",
    tint: "#ff2d55",
    group: "shell",
    title: () => tr("Sound"),
    summary: () => tr("Volume steps, hearing protection, and the media tab"),
    paths: ["audio", "sidebar.left.media"],
  },
  {
    id: "search",
    icon: "search",
    tint: "#8e8e93",
    group: "features",
    title: () => tr("Search"),
    summary: () => tr("The overview and launcher, and what it may search"),
    paths: ["overview"],
  },
  {
    id: "screenshots",
    icon: "screenshot_region",
    tint: "#8e8e93",
    group: "features",
    title: () => tr("Screenshots"),
    summary: () => tr("Where captures go, and reading text from them"),
    paths: ["capture"],
  },
  {
    id: "tools",
    icon: "handyman",
    tint: "#ff9500",
    group: "features",
    title: () => tr("Shelf and overlay"),
    summary: () => tr("The drop shelf, and the always-on-top overlay"),
    paths: ["shelf", "overlay"],
  },
  {
    id: "power",
    icon: "power_settings_new",
    tint: "#ff9500",
    group: "features",
    title: () => tr("Power"),
    summary: () => tr("Which actions the session screen offers"),
    paths: ["session"],
  },
  {
    id: "ai",
    icon: "neurology",
    tint: "#af52de",
    group: "features",
    title: () => tr("AI and translation"),
    summary: () => tr("The assistant, and the translator in the left sidebar"),
    paths: ["ai", "policies.ai", "sidebar.left.translator"],
  },
  {
    id: "keyboard",
    icon: "keyboard",
    tint: "#8e8e93",
    group: "features",
    title: () => tr("Keyboard"),
    summary: () => tr("The keys that open each part of the shell"),
    paths: ["keybinds"],
  },
  {
    id: "privacy",
    icon: "shield",
    tint: "#0a84ff",
    group: "other",
    title: () => tr("Privacy and safety"),
    summary: () => tr("What is safe to show at work, and image search"),
    paths: ["workSafety", "policies.weeb", "sidebar.left.booru"],
  },
  {
    id: "windows",
    icon: "desktop_windows",
    tint: "#8e8e93",
    group: "other",
    title: () => tr("Window managers"),
    summary: () => tr("GlazeWM or komorebi, for the workspaces widget"),
    paths: ["windows.windowManager", "windows.glazewm", "windows.komorebi"],
  },
  {
    id: "advanced",
    icon: "build",
    tint: "#8e8e93",
    group: "other",
    title: () => tr("Advanced"),
    summary: () => tr("Workarounds with a cost, and how often stats are read"),
    paths: ["hacks", "resources"],
    caution: () =>
      tr(
        "Everything under hacks reaches past what Windows offers a shell. Each one says what it costs.",
      ),
  },
  {
    id: "presets",
    icon: "bookmarks",
    tint: "#ffcc00",
    group: "other",
    title: () => tr("Presets"),
    summary: () => tr("The whole setup saved under a name"),
    paths: [],
    custom: "presets",
  },
];

/** Whether `path` is `prefix` or sits under it. */
const under = (path: string, prefix: string) =>
  path === prefix || path.startsWith(`${prefix}.`);

/** Where each widget sits and what it is called are decided by dragging it in
 *  edit mode, not typed here. */
const PLACED_BY_EDIT_MODE = /^background\.widgets\.\w+\.(id|x|y)$/;

/** The page a field belongs on — the longest claim on its path — or nothing,
 *  for a field that is not shown or that the table has forgotten. */
export function pageFor(field: Field): SettingsPage | undefined {
  if (PLACED_BY_EDIT_MODE.test(field.path)) return undefined;
  let best: SettingsPage | undefined;
  let length = 0;
  for (const page of PAGES) {
    for (const prefix of page.paths) {
      if (under(field.path, prefix) && prefix.length > length) {
        best = page;
        length = prefix.length;
      }
    }
  }
  return best;
}

/** The position of a field's claim on its page, which is the order the
 *  page shows its rows in. */
export function orderOn(page: SettingsPage, field: Field): number {
  let found = -1;
  let length = 0;
  page.paths.forEach((prefix, index) => {
    if (under(field.path, prefix) && prefix.length > length) {
      found = index;
      length = prefix.length;
    }
  });
  return found;
}
