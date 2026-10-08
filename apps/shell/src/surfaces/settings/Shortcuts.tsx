// The Keyboard page, laid out as the Mac's is: a switch for the whole thing,
// and a "Keyboard Shortcuts…" button opening a sheet with the shortcuts
// sorted by what they open on the left, and a row each on the right — a box
// to turn it off, its name, and the keys, recorded by pressing them.
//
// Turning one off writes an empty chord, which the hotkey service skips
// (`services/hotkeys.rs`); turning it back on puts the default back. What
// Windows refused, or what two shortcuts share, comes from the same key
// report the first-run wizard reads, with the same suggested replacement.

import { useEffect, useState } from "react";
import type { Config, KeyStatus } from "@bw/core";
import { defaultConfig } from "@bw/core";
import { Button, Check, Switch, Symbol } from "../../widgets";
import { tr } from "../../i18n";
import { actions } from "../../shell/store";
import { explain } from "../wizard/Wizard";
import { capsOf, chordFrom } from "./chord";

type Binding = Exclude<keyof Config["keybinds"], "enable">;

/** The sheet's left column: what each shortcut opens, grouped as the pages
 *  that hold those features are. English, translated when drawn. */
const CATEGORIES: { title: string; icon: string; bindings: Binding[] }[] = [
  {
    title: "Sidebars",
    icon: "view_sidebar",
    bindings: ["sidebarLeft", "sidebarRight"],
  },
  {
    title: "Wallpaper and desktop",
    icon: "photo_library",
    bindings: ["wallpaperSelector", "widgetEditMode", "desktopMenu"],
  },
  {
    title: "Screenshots",
    icon: "screenshot_region",
    bindings: ["captureRegion", "captureOcr", "captureTranslate"],
  },
  { title: "Search", icon: "search", bindings: ["overview"] },
  {
    title: "Shelf and overlay",
    icon: "handyman",
    bindings: ["shelf", "overlay"],
  },
  { title: "Power", icon: "power_settings_new", bindings: ["session"] },
  { title: "Settings", icon: "settings", bindings: ["settings"] },
];

/** What each shortcut does, said as the row's name. */
const NAMES: Record<Binding, string> = {
  sidebarLeft: "Open the left sidebar",
  sidebarRight: "Open the right sidebar",
  wallpaperSelector: "Choose a wallpaper",
  widgetEditMode: "Arrange the desktop widgets",
  desktopMenu: "Open the desktop menu",
  captureRegion: "Capture part of the screen",
  captureOcr: "Read the text in part of the screen",
  captureTranslate: "Translate part of the screen",
  overview: "Open the overview and launcher",
  shelf: "Open the shelf",
  overlay: "Open the overlay",
  session: "Open the session screen",
  settings: "Open settings",
};

export function Shortcuts({
  config,
  onSet,
}: {
  config: Config;
  onSet: (path: string, value: unknown) => void;
}) {
  const [open, setOpen] = useState(false);

  return (
    <>
      <section className="bw-settings-card">
        <div className="bw-settings-card-rows">
          <div className="bw-settings-row">
            <div className="bw-settings-label" title="keybinds.enable">
              <span>{tr("Use keyboard shortcuts")}</span>
              <em>{tr("Off, none of the keys below do anything.")}</em>
            </div>
            <div className="bw-settings-control">
              <Switch
                checked={config.keybinds.enable}
                label={tr("Use keyboard shortcuts")}
                onChange={(next) => onSet("keybinds.enable", next)}
              />
            </div>
          </div>
          <div className="bw-settings-row">
            <div className="bw-settings-label">
              <span>{tr("Keyboard shortcuts")}</span>
              <em>{tr("Which keys open each part of the shell.")}</em>
            </div>
            <div className="bw-settings-control">
              <Button onClick={() => setOpen(true)}>
                {tr("Keyboard Shortcuts…")}
              </Button>
            </div>
          </div>
        </div>
      </section>
      {open ? (
        <Sheet
          keybinds={config.keybinds}
          onSet={onSet}
          onClose={() => setOpen(false)}
        />
      ) : null}
    </>
  );
}

function Sheet({
  keybinds,
  onSet,
  onClose,
}: {
  keybinds: Config["keybinds"];
  onSet: (path: string, value: unknown) => void;
  onClose: () => void;
}) {
  const [category, setCategory] = useState(0);
  const [recording, setRecording] = useState<Binding | null>(null);
  const [report, setReport] = useState<KeyStatus[]>([]);

  // Asked again whenever a chord changes: the shell re-registers the keys on
  // every config write, and what Windows refused can only be known after.
  const chords = JSON.stringify(keybinds);
  useEffect(() => {
    const timer = window.setTimeout(() => {
      actions.keyReport().then(setReport, () => setReport([]));
    }, 400);
    return () => window.clearTimeout(timer);
  }, [chords]);

  // Recording takes the next chord pressed anywhere in the window. Escape
  // gives up rather than being recorded, as on a Mac; when nothing is being
  // recorded it closes the sheet.
  useEffect(() => {
    const listen = (event: KeyboardEvent) => {
      if (!recording) {
        if (event.key === "Escape") onClose();
        return;
      }
      event.preventDefault();
      event.stopPropagation();
      if (event.key === "Escape") {
        setRecording(null);
        return;
      }
      // ponytail: a few Win chords (Win+X, Win+L) are taken by Windows
      // before any window sees them, so they cannot be recorded here — and
      // Windows would refuse to hand them over anyway.
      const chord = chordFrom(event);
      if (!chord) return;
      onSet(`keybinds.${recording}`, chord);
      setRecording(null);
    };
    window.addEventListener("keydown", listen, true);
    return () => window.removeEventListener("keydown", listen, true);
  }, [recording, onSet, onClose]);

  const restore = () => {
    for (const { bindings } of CATEGORIES) {
      for (const binding of bindings) {
        if (keybinds[binding] !== defaultConfig.keybinds[binding]) {
          onSet(`keybinds.${binding}`, defaultConfig.keybinds[binding]);
        }
      }
    }
  };

  const shown = CATEGORIES[category]!;

  return (
    <div
      className="bw-shortcuts-scrim"
      onMouseDown={(event) => {
        if (event.target === event.currentTarget) onClose();
      }}
    >
      <div
        className="bw-shortcuts"
        role="dialog"
        aria-label={tr("Keyboard Shortcuts")}
      >
        <div className="bw-shortcuts-body">
          <nav className="bw-shortcuts-categories">
            {CATEGORIES.map((entry, index) => (
              <button
                key={entry.title}
                type="button"
                className={index === category ? "selected" : ""}
                onClick={() => {
                  setRecording(null);
                  setCategory(index);
                }}
              >
                <Symbol name={entry.icon} size={18} />
                <span>{tr(entry.title)}</span>
              </button>
            ))}
          </nav>
          <ul className="bw-shortcuts-list">
            {shown.bindings.map((binding) => {
              const chord = keybinds[binding];
              const status = report.find((key) => key.binding === binding);
              const trouble =
                chord &&
                status &&
                (status.refused ||
                  status.takenByWindows ||
                  status.sharedWith.length > 0)
                  ? status
                  : null;
              return (
                <li key={binding}>
                  <Check
                    checked={chord !== ""}
                    ariaLabel={tr(NAMES[binding])}
                    onChange={(on) =>
                      onSet(
                        `keybinds.${binding}`,
                        on ? defaultConfig.keybinds[binding] : "",
                      )
                    }
                  />
                  <div className="bw-shortcuts-name">
                    <span>{tr(NAMES[binding])}</span>
                    {trouble ? (
                      <em>
                        <Symbol name="warning" size={14} />
                        {explain(trouble)}
                        {trouble.suggestion ? (
                          <button
                            type="button"
                            onClick={() =>
                              onSet(`keybinds.${binding}`, trouble.suggestion)
                            }
                          >
                            {tr("Use %1").replace("%1", trouble.suggestion)}
                          </button>
                        ) : null}
                      </em>
                    ) : null}
                  </div>
                  <button
                    type="button"
                    className="bw-shortcuts-keys"
                    data-recording={recording === binding}
                    disabled={chord === ""}
                    aria-label={tr("Change the keys for %1").replace(
                      "%1",
                      tr(NAMES[binding]),
                    )}
                    onClick={() =>
                      setRecording(recording === binding ? null : binding)
                    }
                  >
                    {recording === binding ? (
                      tr("Press the keys…")
                    ) : chord ? (
                      capsOf(chord).map((cap, index) => (
                        <kbd key={index}>{cap}</kbd>
                      ))
                    ) : (
                      <span>{tr("Off")}</span>
                    )}
                  </button>
                </li>
              );
            })}
          </ul>
        </div>
        <footer className="bw-shortcuts-foot">
          <Button variant="text" onClick={restore}>
            {tr("Restore Defaults")}
          </Button>
          <Button variant="filled" onClick={onClose}>
            {tr("Done")}
          </Button>
        </footer>
      </div>
    </div>
  );
}
