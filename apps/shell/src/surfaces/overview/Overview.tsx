// The search overlay.
//
// The original's overview is a full-screen panel with a search bar over a
// workspace grid. The grid is not built here — live window previews need
// `DwmRegisterThumbnail` and are their own piece of work — so what is left is
// the part people actually use: type, and the thing you meant is the first
// row.
//
// The ordering is decided in `bw-core` and arrives already sorted. This file
// draws it and handles the keyboard, and deliberately does no ranking of its
// own: two ranking implementations would disagree the first time either was
// touched.

import { useCallback, useEffect, useRef, useState } from "react";
import type { LauncherResult } from "@bw/core";
import { Symbol } from "../../widgets";
import { tr } from "../../i18n";
import { actions, connect, useShell } from "../../shell/store";
import { describeError } from "../../shell/errors";
import { backend } from "../../shell/backend";
import "./overview.css";

/** What each `/` action does, in the shell rather than on the machine.
 *
 * The keywords come from `bw-core` — they are what the user types, so they are
 * English there and matched there. What each one means is a sentence, so it is
 * written here where it can be translated. */
const ACTIONS: Record<
  string,
  { run: () => void | Promise<unknown>; describe: () => string }
> = {
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
};

/** What a row says under its title when the backend left that blank.
 *
 * Without this a command row is the line the user just typed shown back at
 * them, and an action row is a bare keyword — neither says what Enter does. */
function describe(row: LauncherResult): string {
  if (row.subtitle) return row.subtitle;
  switch (row.kind) {
    case "command":
      return tr("Run command");
    case "webSearch":
      return tr("Search the web");
    case "action":
      return ACTIONS[row.payload]?.describe() ?? "";
    default:
      return "";
  }
}

export function Overview() {
  const ready = useShell((state) => state.ready);
  const open = useShell((state) => state.states.overviewOpen);
  const enabled = useShell((state) => state.config.overview.enable);
  const scanned = useShell((state) => state.appsScanned);

  const [query, setQuery] = useState("");
  const [rows, setRows] = useState<LauncherResult[]>([]);
  const [selected, setSelected] = useState(0);
  const [failure, setFailure] = useState<string | null>(null);
  const field = useRef<HTMLInputElement>(null);

  useEffect(() => {
    void connect();
  }, []);

  // Opening is a fresh start: the last search is not what this one is about.
  // The window is never destroyed, only hidden, so this is the only thing that
  // clears the box between one use and the next.
  useEffect(() => {
    setQuery("");
    setSelected(0);
    setFailure(null);
    // Shown by the backend, so the element is not focusable until after the
    // frame it appears in.
    const frame = requestAnimationFrame(() => field.current?.focus());
    return () => cancelAnimationFrame(frame);
  }, [open]);

  // Re-queried per keystroke, and again when the application scan lands —
  // the overview opens before it finishes.
  useEffect(() => {
    let stale = false;
    void actions
      .launcherResults(query)
      .then((found) => {
        // A slow query for an earlier keystroke must not overwrite the
        // results for what is in the box now.
        if (stale) return;
        setRows(found);
        setSelected(0);
      })
      .catch(() => {
        if (!stale) setRows([]);
      });
    return () => {
      stale = true;
    };
  }, [query, scanned]);

  const close = useCallback(() => actions.setState("overviewOpen", false), []);

  const activate = useCallback(
    async (row: LauncherResult) => {
      // Closed first: whatever starts should get the focus, and a launcher
      // still on screen behind it would be holding it.
      await close();
      try {
        switch (row.kind) {
          case "window":
            await actions.activateWindow(row.payload, false);
            break;
          case "app":
            await actions.launchEntry(row.payload, row.appKind ?? "shortcut");
            break;
          case "command":
            await actions.runCommand(row.payload);
            break;
          case "webSearch":
            await actions.openUrl(row.payload);
            break;
          case "calculator":
            await copy(row.payload);
            break;
          case "action":
            await ACTIONS[row.payload]?.run();
            break;
        }
      } catch (error) {
        // Every one of these can be refused — Windows declines foreground
        // changes, a command names something that is not there. Saying so
        // beats a launcher that closed and did nothing.
        setFailure(describeError(error));
        await actions.setState("overviewOpen", true);
      }
    },
    [close],
  );

  const onKeyDown = (event: React.KeyboardEvent) => {
    switch (event.key) {
      case "Escape":
        event.preventDefault();
        void close();
        break;
      case "ArrowDown":
        event.preventDefault();
        setSelected((index) => (rows.length ? (index + 1) % rows.length : 0));
        break;
      case "ArrowUp":
        event.preventDefault();
        setSelected((index) =>
          rows.length ? (index - 1 + rows.length) % rows.length : 0,
        );
        break;
      case "Enter": {
        event.preventDefault();
        const row = rows[selected];
        if (row) void activate(row);
        break;
      }
    }
  };

  if (!ready || !enabled) return null;

  return (
    <div
      className="bw-overview"
      // Clicking past the panel is the other way out, as with every overlay.
      onMouseDown={(event) => {
        if (event.target === event.currentTarget) void close();
      }}
    >
      <div className="bw-overview-panel">
        <label className="bw-overview-field">
          <Symbol name="search" size={22} />
          <input
            ref={field}
            value={query}
            placeholder={tr("Search applications, windows and the web")}
            onChange={(event) => setQuery(event.target.value)}
            onKeyDown={onKeyDown}
            spellCheck={false}
            autoComplete="off"
          />
        </label>

        {failure ? <p className="bw-overview-failure">{failure}</p> : null}

        {rows.length ? (
          <ul className="bw-overview-results">
            {rows.map((row, index) => (
              <Row
                key={`${row.kind}:${row.payload}:${index}`}
                row={row}
                selected={index === selected}
                onHover={() => setSelected(index)}
                onPressEnter={() => void activate(row)}
              />
            ))}
          </ul>
        ) : (
          <p className="bw-overview-empty">
            {query ? tr("Nothing matched") : tr("Start typing")}
          </p>
        )}
      </div>
    </div>
  );
}

interface RowProps {
  row: LauncherResult;
  selected: boolean;
  onHover: () => void;
  onPressEnter: () => void;
}

function Row({ row, selected, onHover, onPressEnter }: RowProps) {
  const element = useRef<HTMLLIElement>(null);
  const subtitle = describe(row);

  // Keyboard selection has to drag the list with it, or holding Down walks
  // the highlight off the bottom of a scrolled list.
  useEffect(() => {
    if (selected) element.current?.scrollIntoView({ block: "nearest" });
  }, [selected]);

  return (
    <li
      ref={element}
      className={[
        "bw-overview-row",
        row.kind === "calculator" ? "answer" : "",
        selected ? "selected" : "",
      ]
        .filter(Boolean)
        .join(" ")}
      onMouseMove={onHover}
      onClick={onPressEnter}
    >
      {row.icon ? (
        <img
          className="bw-overview-icon"
          src={backend().assetUrl(row.icon)}
          alt=""
          draggable={false}
        />
      ) : (
        <Symbol name={row.symbol} size={24} className="bw-overview-glyph" />
      )}

      <span className="bw-overview-text">
        <span className="bw-overview-title">
          <Highlighted text={row.title} positions={row.positions} />
        </span>
        {subtitle ? (
          <span className="bw-overview-subtitle">{subtitle}</span>
        ) : null}
      </span>

      {selected ? (
        <Symbol
          name="keyboard_return"
          size={18}
          className="bw-overview-enter"
        />
      ) : null}
    </li>
  );
}

/** Marks the characters the query matched.
 *
 * The positions count characters, not UTF-16 units, so the text is split with
 * `Array.from` — indexing a string directly would put the marks in the wrong
 * place the moment a name contains anything outside the basic plane. */
function Highlighted({
  text,
  positions,
}: {
  text: string;
  positions: number[];
}) {
  if (!positions.length) return <>{text}</>;

  const characters = Array.from(text);
  const marked = new Set(positions);
  const parts: React.ReactNode[] = [];
  let run = "";
  let runMarked = false;

  const flush = () => {
    if (!run) return;
    parts.push(
      runMarked ? (
        <mark key={parts.length}>{run}</mark>
      ) : (
        <span key={parts.length}>{run}</span>
      ),
    );
    run = "";
  };

  characters.forEach((character, index) => {
    const isMarked = marked.has(index);
    if (isMarked !== runMarked) {
      flush();
      runMarked = isMarked;
    }
    run += character;
  });
  flush();

  return <>{parts}</>;
}

/** Copies an arithmetic answer, if the webview will allow it. */
async function copy(value: string): Promise<void> {
  try {
    await navigator.clipboard.writeText(value);
  } catch {
    // Clipboard access can be refused, and there is nothing useful to say
    // about it: the answer is on screen either way.
  }
}
