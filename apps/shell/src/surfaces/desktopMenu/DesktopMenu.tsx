// The menu the desktop's right button opens.
//
// A small panel on a transparent full-screen sheet, which is how a context
// menu has to be built when every surface is its own window: there is nowhere
// smaller to put it, and the sheet is what catches the click that dismisses it.
//
// The one thing this file is careful about is where the panel goes. It does
// not work that out — the flip-at-the-edge rule is in `bw-core` under tests —
// but it does have to measure what it drew before it can ask, and it must not
// show the menu in the wrong place first. So the first paint is invisible, the
// measurement happens on it, and the panel appears already positioned.

import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
} from "react";
import type { MenuItem } from "@bw/core";
import { Symbol } from "../../widgets";
import { tr } from "../../i18n";
import { actions, connect, useShell } from "../../shell/store";
import { describeError } from "../../shell/errors";
import "./desktopMenu.css";

/** What each entry says, in the user's language. */
function label(item: MenuItem): string {
  switch (item) {
    case "changeWallpaper":
      return tr("Change wallpaper");
    case "nextWallpaper":
      return tr("Next wallpaper");
    case "editWidgets":
      return tr("Edit widgets");
    case "overview":
      return tr("Search");
    case "screenshot":
      return tr("Screenshot");
    case "session":
      return tr("Session");
    case "displaySettings":
      return tr("Display settings");
    case "personalise":
      return tr("Personalise");
  }
}

/** Which glyph, mirroring `MenuItem::symbol` in `bw-core`. */
function symbol(item: MenuItem): string {
  switch (item) {
    case "changeWallpaper":
      return "wallpaper";
    case "nextWallpaper":
      return "shuffle";
    case "editWidgets":
      return "widgets";
    case "overview":
      return "search";
    case "screenshot":
      return "photo_camera";
    case "session":
      return "power_settings_new";
    case "displaySettings":
      return "desktop_windows";
    case "personalise":
      return "palette";
  }
}

/** Whether picking this leaves the shell for Windows. Mirrors `bw-core`. */
function leavesTheShell(item: MenuItem): boolean {
  return item === "displaySettings" || item === "personalise";
}

export function DesktopMenu() {
  const ready = useShell((state) => state.ready);
  const open = useShell((state) => state.states.desktopMenuOpen);
  const enabled = useShell((state) => state.config.desktopMenu.enable);

  const [items, setItems] = useState<MenuItem[]>([]);
  const [at, setAt] = useState<{ x: number; y: number } | null>(null);
  const [focused, setFocused] = useState(0);
  const [problem, setProblem] = useState<string | null>(null);
  const panel = useRef<HTMLDivElement>(null);

  useEffect(() => {
    void connect();
  }, []);

  // Re-asked on every open rather than once: the entries follow the config,
  // and the config can be edited while the shell is running.
  //
  // Both the entries and the position are cleared first. Keeping the previous
  // open's list would mean measuring a menu that is about to change size, and
  // the measurement is what the position is worked out from.
  useEffect(() => {
    setItems([]);
    setAt(null);
    if (!open) return;
    setProblem(null);
    setFocused(0);
    void actions
      .desktopMenuItems()
      .then(setItems)
      .catch(() => setItems([]));
  }, [open]);

  // After the invisible first paint, and before the browser shows anything:
  // measure, ask, and only then let it be seen.
  useLayoutEffect(() => {
    if (!open || at || !items.length) return;
    const element = panel.current;
    if (!element) return;

    let stale = false;
    void actions
      .placeDesktopMenu(element.offsetWidth, element.offsetHeight)
      .then((placement) => {
        if (!stale) setAt(placement);
      })
      // Somewhere is better than nowhere: an unplaceable menu still opens, in
      // the corner, rather than never appearing.
      .catch(() => {
        if (!stale) setAt({ x: 8, y: 8 });
      });
    return () => {
      stale = true;
    };
  }, [open, at, items]);

  const close = useCallback(() => actions.toggleDesktopMenu("close"), []);

  const pick = useCallback(async (item: MenuItem) => {
    try {
      await actions.runDesktopMenuItem(item);
    } catch (error) {
      // The settings pages can be refused — they were reorganised more than
      // once, and the page this asks for may not exist on this Windows.
      setProblem(describeError(error));
      await actions.toggleDesktopMenu("open");
    }
  }, []);

  const onKeyDown = useCallback(
    (event: React.KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        void close();
        return;
      }
      if (!items.length) return;

      if (event.key === "ArrowDown" || event.key === "ArrowUp") {
        event.preventDefault();
        const step = event.key === "ArrowDown" ? 1 : -1;
        setFocused((index) => (index + step + items.length) % items.length);
        return;
      }
      if (event.key === "Enter" || event.key === " ") {
        event.preventDefault();
        const item = items[focused];
        if (item) void pick(item);
      }
    },
    [close, focused, items, pick],
  );

  // Nothing to draw until the entries are known: a menu is its entries, and
  // an empty panel would be measured and placed and then have to move.
  if (!ready || !enabled || !open || !items.length) return null;

  // Where the rule goes: in front of the first entry that hands the user over
  // to Windows. `bw-core` keeps those contiguous at the end, so one is enough.
  const firstLeaver = items.findIndex(leavesTheShell);

  return (
    <div
      className="bw-desktop-menu"
      role="dialog"
      aria-label={tr("Desktop menu")}
      tabIndex={-1}
      ref={(element) => element?.focus()}
      onKeyDown={onKeyDown}
      onMouseDown={(event) => {
        if (event.target === event.currentTarget) void close();
      }}
      // A right-click on the sheet closes it too, rather than stacking a
      // second menu on the first.
      onContextMenu={(event) => {
        event.preventDefault();
        void close();
      }}
    >
      <div
        ref={panel}
        className="bw-desktop-menu-panel"
        style={
          at
            ? { left: at.x, top: at.y }
            : // The measuring pass. Laid out exactly as it will be drawn, so
              // the size is the real one, but not yet anywhere in particular.
              { left: 0, top: 0, visibility: "hidden" }
        }
      >
        {problem ? <p className="bw-desktop-menu-problem">{problem}</p> : null}

        <ul className="bw-desktop-menu-items">
          {items.map((item, index) => (
            <li key={item}>
              {index === firstLeaver && index > 0 ? (
                <hr className="bw-desktop-menu-rule" />
              ) : null}
              <button
                type="button"
                className={[
                  "bw-desktop-menu-item",
                  index === focused ? "focused" : "",
                ]
                  .filter(Boolean)
                  .join(" ")}
                onMouseEnter={() => setFocused(index)}
                onClick={() => void pick(item)}
              >
                <Symbol name={symbol(item)} size={20} />
                <span>{label(item)}</span>
              </button>
            </li>
          ))}
        </ul>
      </div>
    </div>
  );
}
