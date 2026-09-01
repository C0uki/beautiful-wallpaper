// What each `/` keyword does.
//
// The keywords themselves come from `bw-core` — they are what the user types,
// so they are matched and ranked there. What each one *means* is a sentence
// and a call, so it lives here, where the sentence can be translated.
//
// Kept out of the component so a test can hold it against the generated list
// of keywords. A keyword offered by the backend with nothing behind it here is
// a row that appears in the launcher, says nothing, and does nothing when it
// is chosen — which is exactly what happened to `/desktop`.

import { tr } from "../../i18n";
import { actions } from "../../shell/store";

export interface LauncherAction {
  run: () => void | Promise<unknown>;
  describe: () => string;
}

export const ACTIONS: Record<string, LauncherAction> = {
  light: {
    run: () => actions.setMode("light"),
    describe: () => tr("Switch to the light theme"),
  },
  dark: {
    run: () => actions.setMode("dark"),
    describe: () => tr("Switch to the dark theme"),
  },
  wallpaper: {
    run: () => actions.setState("wallpaperSelectorOpen", true),
    describe: () => tr("Open the wallpaper picker"),
  },
  random: {
    run: () => actions.randomWallpaper(),
    describe: () => tr("Pick another wallpaper"),
  },
  widgets: {
    run: () => actions.toggleState("widgetEditMode"),
    describe: () => tr("Rearrange the desktop widgets"),
  },
  sidebar: {
    run: () => actions.setState("sidebarRightOpen", true),
    describe: () => tr("Open the control centre"),
  },
  screenshot: {
    run: () => actions.startCapture("screenshot"),
    describe: () => tr("Pick a region and save it"),
  },
  ocr: {
    run: () => actions.startCapture("ocr"),
    describe: () => tr("Read the text in a region"),
  },
  translate: {
    run: () => actions.startCapture("translate"),
    describe: () => tr("Read a region and translate it"),
  },
  session: {
    run: () => actions.setState("sessionOpen", true),
    describe: () => tr("Lock, sleep, restart or shut down"),
  },
  desktop: {
    run: () => actions.toggleDesktopMenu("open"),
    describe: () => tr("Open the desktop menu"),
  },
  shelf: {
    run: () => actions.setState("shelfOpen", true),
    describe: () => tr("Open the drop shelf"),
  },
  overlay: {
    run: () => actions.setState("overlayOpen", true),
    describe: () => tr("Open the floating overlay"),
  },
};
