// Which config sections appear on which settings page.
//
// The rows themselves are generated from the Rust schema, so nothing here has
// to list a key — only where each *section* belongs and in what order. That is
// the one thing the schema genuinely cannot say.
//
// A section missing from this table would be a set of settings with no page to
// live on, which is exactly the silent failure the generated form exists to
// avoid, so `pages.test.ts` holds every section in the schema against it.

import { tr } from "../../i18n";

export interface SettingsPage {
  id: string;
  icon: string;
  title: () => string;
  /** Top-level config sections, in the order they appear on the page. */
  sections: string[];
  /** Shown above the first section, when the page needs a warning. */
  caution?: () => string;
}

export const PAGES: SettingsPage[] = [
  {
    id: "general",
    icon: "browse",
    title: () => tr("General"),
    sections: ["language", "time", "weather", "policies", "workSafety"],
  },
  {
    id: "appearance",
    icon: "palette",
    title: () => tr("Appearance"),
    sections: ["appearance"],
  },
  {
    id: "desktop",
    icon: "texture",
    title: () => tr("Desktop"),
    sections: ["background", "wallpaperSelector"],
  },
  {
    id: "bar",
    icon: "toast",
    title: () => tr("Bar and dock"),
    sections: ["bar", "dock"],
  },
  {
    id: "panels",
    icon: "dock_to_left",
    title: () => tr("Panels"),
    sections: ["sidebar", "overview", "notifications", "osd"],
  },
  {
    id: "overlays",
    icon: "layers",
    title: () => tr("Overlays"),
    sections: ["overlay", "desktopMenu", "shelf", "capture", "session"],
  },
  {
    id: "keys",
    icon: "keyboard",
    title: () => tr("Keys"),
    sections: ["keybinds"],
  },
  {
    id: "services",
    icon: "settings",
    title: () => tr("Services"),
    sections: ["ai", "audio", "resources"],
  },
  {
    id: "windows",
    icon: "desktop_windows",
    title: () => tr("Windows"),
    sections: ["windows", "hacks"],
    caution: () =>
      tr(
        "Everything under hacks reaches past what Windows offers a shell. Each one says what it costs.",
      ),
  },
];

/** The page a section belongs on, or nothing if the table has forgotten it. */
export function pageFor(section: string): SettingsPage | undefined {
  return PAGES.find((page) => page.sections.includes(section));
}
