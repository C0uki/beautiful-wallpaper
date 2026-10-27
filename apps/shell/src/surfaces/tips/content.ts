// What the tips say.
//
// Written in English and translated as they are drawn, as everything else in
// the shell is. The keyboard's tips are not here: they are the config's own
// keys, read as the page is drawn, so changing a shortcut changes its tip.

import type { StateFlagName } from "@bw/core";

export interface Tip {
  icon: string;
  title: string;
  body: string;
  /** A command to type, shown so it can be copied. */
  code?: string;
  /** A button that opens what the tip is about. */
  open?: { flag: StateFlagName; label: string };
}

export interface Collection {
  id: string;
  icon: string;
  /** The tile's colour, fixed as the settings pages' are. */
  tint: string;
  title: string;
  summary: string;
  tips: Tip[];
}

export const KEYBOARD = "keyboard";

export const COLLECTIONS: Collection[] = [
  {
    id: "basics",
    icon: "lightbulb",
    tint: "#ff9500",
    title: "Using the shell",
    summary: "The things it can do that are easy to miss",
    tips: [
      {
        icon: "dock_to_bottom",
        title: "Drop files on the Dock",
        body: "Drag files onto an app's icon in the Dock to open them with that app.",
      },
      {
        icon: "photo_library",
        title: "Choose a wallpaper",
        body: "The picker shows your Pictures folder and wallpapers from the web. The colours of the whole shell follow the wallpaper you choose.",
        open: { flag: "wallpaperSelectorOpen", label: "Choose a wallpaper" },
      },
      {
        icon: "shuffle",
        title: "Another wallpaper",
        body: "The shuffle button at the bottom right of the desktop picks another wallpaper from the same folder.",
      },
      {
        icon: "widgets",
        title: "Arrange the widgets",
        body: "In edit mode, drag the clock, the weather and the others wherever you like. They snap to a grid.",
        open: { flag: "widgetEditMode", label: "Edit widgets" },
      },
      {
        icon: "view_sidebar",
        title: "Two sidebars",
        body: "The right one has quick toggles, notifications, a calendar and a to-do list. The left one has the assistant, the translator and the media player.",
        open: { flag: "sidebarRightOpen", label: "Open the right sidebar" },
      },
      {
        icon: "search",
        title: "Search everything",
        body: "Search opens apps, files and settings, and works out sums as you type them.",
        open: { flag: "overviewOpen", label: "Open search" },
      },
      {
        icon: "screenshot_region",
        title: "Capture, read and translate",
        body: "Pick part of the screen to save it, read the text in it, or translate that text. The keys for each are in the keyboard tips.",
      },
      {
        icon: "inbox",
        title: "Keep files on the shelf",
        body: "Drop files on the shelf to keep them at hand, then drag them out wherever they are needed.",
        open: { flag: "shelfOpen", label: "Open the shelf" },
      },
      {
        icon: "save",
        title: "Save a setup as a preset",
        body: "In Settings, save the whole configuration under a name, and put it back with one press.",
        open: { flag: "settingsOpen", label: "Open settings" },
      },
      {
        icon: "bedtime",
        title: "Night light",
        body: "Warms the screen in the evening. Turn it on from the quick toggles in the right sidebar.",
      },
      {
        icon: "desktop_windows",
        title: "More than one monitor",
        body: "The wallpaper is drawn on every monitor, and the sidebars, search and menus open on the one the pointer is on.",
      },
    ],
  },
  {
    id: KEYBOARD,
    icon: "keyboard",
    tint: "#8e8e93",
    title: "Keyboard",
    summary: "The keys that open each part of the shell",
    tips: [],
  },
  {
    id: "trouble",
    icon: "warning",
    tint: "#ff3b30",
    title: "When something goes wrong",
    summary: "The way back, and what is working as it should",
    tips: [
      {
        icon: "toolbar",
        title: "The taskbar is gone",
        body: "If the shell was closed without putting the Windows taskbar back, press Ctrl+Shift+Esc, choose Run new task, and run this.",
        code: "bw taskbar show",
      },
      {
        icon: "keyboard",
        title: "A key does nothing",
        body: "Windows keeps some keys for itself and will not hand them over. Choose other keys for that shortcut in Settings.",
        open: { flag: "settingsOpen", label: "Open settings" },
      },
      {
        icon: "autorenew",
        title: "Waiting for an update",
        body: "The shell looks for a new version a minute after it starts and every two hours after that. Starting it again looks straight away.",
      },
      {
        icon: "dock_to_bottom",
        title: "The taskbar at the top",
        body: "While the shell hides the Windows taskbar, it moves it to the bottom, where hiding it gives the screen back. It goes back to the top when the shell stops.",
      },
      {
        icon: "restart_alt",
        title: "Start again",
        body: "This stops the shell. Run bw from the Run dialog to start it again.",
        code: "bw quit",
      },
    ],
  },
  {
    id: "commands",
    icon: "terminal",
    tint: "#34c759",
    title: "The bw command",
    summary: "What can be typed into the Run dialog",
    tips: [
      {
        icon: "terminal",
        title: "Type bw anywhere",
        body: "Every bw command works from the Run dialog (Win+R) and from a terminal. This lists them all.",
        code: "bw --help",
      },
      {
        icon: "shuffle",
        title: "Another wallpaper",
        body: "Picks another wallpaper from the same folder.",
        code: "bw wallpapers random",
      },
      {
        icon: "screenshot_region",
        title: "Capture part of the screen",
        body: "Opens the region picker on the monitor the pointer is on.",
        code: "bw capture region",
      },
      {
        icon: "tune",
        title: "Change one setting",
        body: "Any setting, by its name in the configuration reference.",
        code: "bw config set bar.vertical true",
      },
      {
        icon: "save",
        title: "Save a setup",
        body: "Saves the whole configuration under a name. bw preset apply puts it back.",
        code: "bw preset save Work",
      },
      {
        icon: "lightbulb",
        title: "Open these tips",
        body: "The same as the desktop menu's Tips.",
        code: "bw tips open",
      },
    ],
  },
];
