// The steps of the first run, in order.
//
// A wizard rather than the original's one scrolling page, because the two are
// answering different questions. On Linux the shell works the moment it
// starts, and the welcome screen is an invitation to make it yours. Here half
// of it may not work at all until somebody looks: no wallpaper has been
// chosen, so the palette has no source; the workspaces widget needs a tiling
// window manager that Windows does not ship; and Windows may have refused any
// of the keyboard shortcuts, silently.
//
// So each step establishes one thing and says what it found. Skipping is
// always allowed — none of this is a form to be completed — but a step that
// found something wrong says so rather than letting the user walk past it.

import { tr } from "../../i18n";

export interface WizardStep {
  id: string;
  icon: string;
  title: () => string;
  /** One line under the title, saying why this step is here at all. */
  blurb: () => string;
}

export const STEPS: WizardStep[] = [
  {
    id: "welcome",
    icon: "waving_hand",
    title: () => tr("Welcome"),
    blurb: () =>
      tr("Two things that change how everything else looks. Both move later."),
  },
  {
    id: "wallpaper",
    icon: "wallpaper",
    title: () => tr("Wallpaper"),
    blurb: () =>
      tr(
        "The colours of every panel are generated from it, so until one is chosen the shell is wearing its fallback palette.",
      ),
  },
  {
    id: "bar",
    icon: "toast",
    title: () => tr("The bar"),
    blurb: () =>
      tr("Where it sits, what shape it is, and what happens to the taskbar."),
  },
  {
    id: "windows",
    icon: "desktop_windows",
    title: () => tr("Workspaces"),
    blurb: () =>
      tr(
        "Windows has no workspaces the shell can read, so this part needs a tiling window manager.",
      ),
  },
  {
    id: "keys",
    icon: "keyboard",
    title: () => tr("Keys"),
    blurb: () =>
      tr(
        "Windows keeps a lot of combinations for itself and refuses the rest without saying so. These were tried just now.",
      ),
  },
  {
    id: "done",
    icon: "check_circle",
    title: () => tr("That's it"),
    blurb: () => tr("Everything here is in Settings, and moves whenever."),
  },
];

/**
 * The step to resume on, given what the state file remembers.
 *
 * A state file written by a build with more steps than this one names a step
 * that does not exist here, and the screen would come up blank on a machine
 * that had merely downgraded.
 */
export function resumeAt(saved: number): number {
  if (!Number.isFinite(saved)) return 0;
  return Math.min(Math.max(Math.trunc(saved), 0), STEPS.length - 1);
}
