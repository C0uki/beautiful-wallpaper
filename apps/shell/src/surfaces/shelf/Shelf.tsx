// The drop shelf.
//
// Files come in from anywhere on the machine and go out to anywhere else. In,
// the page takes the drop like any page, but a web page never learns where a
// dropped file lives: it hands the files to WebView2, which tells the shell
// their paths, and they come back as `ShelfDropped`. In the development
// harness there is no WebView2, and the names are all there is.
//
// Dragging back out cannot be started by the page either — an application
// expecting a file wants shell items, not a web drag — so a press and a few
// pixels of movement hand the selection to the backend, which runs the real
// thing.

import { useCallback, useEffect, useRef, useState } from "react";
import type { DropOutcome, ShelfItem, ShelfKind } from "@bw/core";
import { Event } from "@bw/core";
import { IconButton, Symbol } from "../../widgets";
import { tr } from "../../i18n";
import { actions, connect, useShell } from "../../shell/store";
import { describeError } from "../../shell/errors";
import { backend } from "../../shell/backend";
import "./shelf.css";

/** WebView2's way for a page to hand the shell objects it cannot describe. */
interface WebView2 {
  postMessageWithAdditionalObjects(message: unknown, objects: FileList): void;
}

/** Far enough that a click is not a drag. The shell's own value, in pixels. */
const DRAG_THRESHOLD = 6;

/** Which glyph, mirroring `ShelfKind::symbol` in `bw-core`. */
function symbol(kind: ShelfKind): string {
  switch (kind) {
    case "folder":
      return "folder";
    case "image":
      return "image";
    case "video":
      return "movie";
    case "audio":
      return "music_note";
    case "document":
      return "description";
    case "archive":
      return "folder_zip";
    case "code":
      return "code";
    case "other":
      return "draft";
  }
}

/** A size a person can read, rather than a number of bytes. */
function readableSize(bytes: number): string {
  const units = ["B", "KB", "MB", "GB", "TB"];
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }
  // Whole bytes stay whole; everything else gets one decimal, which is as much
  // precision as a file size is ever read to.
  return `${unit === 0 ? value : value.toFixed(1)} ${units[unit]}`;
}

/** What a drop did, as a sentence — or nothing, when it all just worked. */
function describeDrop(outcome: DropOutcome): string | null {
  if (outcome.refused > 0) {
    return tr("The shelf is full: %1 not added").replace(
      "%1",
      String(outcome.refused),
    );
  }
  return null;
}

export function Shelf() {
  const ready = useShell((state) => state.ready);
  const open = useShell((state) => state.states.shelfOpen);
  const enabled = useShell((state) => state.config.shelf.enable);

  const [items, setItems] = useState<ShelfItem[]>([]);
  const [selected, setSelected] = useState<number[]>([]);
  const [hovering, setHovering] = useState(false);
  const [problem, setProblem] = useState<string | null>(null);
  const press = useRef<{ id: number; x: number; y: number } | null>(null);

  const receive = useCallback(async (paths: string[]) => {
    if (!paths.length) return;
    try {
      const outcome = await actions.addToShelf(paths);
      setProblem(describeDrop(outcome));
    } catch (error) {
      setProblem(describeError(error));
    }
  }, []);

  useEffect(() => {
    void connect();
    void actions
      .shelfItems()
      .then(setItems)
      .catch(() => setItems([]));

    const stop: Array<() => void> = [];
    const api = backend();
    void api
      .listen<ShelfItem[]>(Event.Shelf, setItems)
      .then((off) => stop.push(off));
    // What a drop on the page turned out to be, paths and all.
    void api
      .listen<string[]>(Event.ShelfDropped, (paths) => void receive(paths))
      .then((off) => stop.push(off));

    return () => {
      for (const off of stop) off();
    };
  }, [receive]);

  // The shelf keeps its selection while it is open and forgets it when it is
  // put away: a selection nobody can see is a selection that will surprise
  // somebody the next time they press a button.
  useEffect(() => {
    if (!open) {
      setSelected([]);
      setProblem(null);
    }
  }, [open]);

  const toggleSelected = (id: number, additive: boolean) => {
    setSelected((current) => {
      if (!additive)
        return current.includes(id) && current.length === 1 ? [] : [id];
      return current.includes(id)
        ? current.filter((other) => other !== id)
        : [...current, id];
    });
  };

  const startDrag = useCallback(
    async (id: number) => {
      // Whatever is selected goes, but a drag that starts on an unselected row
      // is about that row — pressing on a file and dragging should never carry
      // four others the user forgot were highlighted.
      const carrying = selected.includes(id) ? selected : [id];
      try {
        await actions.dragFromShelf(carrying);
      } catch (error) {
        setProblem(describeError(error));
      }
    },
    [selected],
  );

  const remove = async (id: number) => {
    try {
      setItems(await actions.removeFromShelf(id));
    } catch (error) {
      setProblem(describeError(error));
    }
    setSelected((current) => current.filter((other) => other !== id));
  };

  const clear = async (missingOnly: boolean) => {
    try {
      setItems(await actions.clearShelf(missingOnly));
      setSelected([]);
      setProblem(null);
    } catch (error) {
      setProblem(describeError(error));
    }
  };

  if (!ready || !enabled || !open) return null;

  const missing = items.filter((item) => item.missing).length;

  return (
    <div
      className={["bw-shelf", hovering ? "hovering" : ""]
        .filter(Boolean)
        .join(" ")}
      onDragOver={(event) => {
        event.preventDefault();
        setHovering(true);
      }}
      onDragLeave={() => setHovering(false)}
      onDrop={(event) => {
        event.preventDefault();
        setHovering(false);
        const files = event.dataTransfer.files;
        const webview = (window as { chrome?: { webview?: WebView2 } }).chrome
          ?.webview;
        // A string, though nothing reads it: Tauri's own handler sees every
        // message first and stops WebView2 handing anything that is not a
        // string to the shell's.
        if (webview) webview.postMessageWithAdditionalObjects("shelf", files);
        else void receive(Array.from(files).map((file) => file.name));
      }}
    >
      <header className="bw-shelf-head">
        <span className="bw-shelf-title">
          <Symbol name="inbox" size={20} />
          {tr("Shelf")}
        </span>
        <span className="bw-shelf-count">
          {items.length ? String(items.length) : ""}
        </span>
        {missing > 0 ? (
          <IconButton
            icon="link_off"
            size={30}
            label="Remove missing files"
            onClick={() => void clear(true)}
          />
        ) : null}
        {/* Not shown on an empty shelf: a button that cannot do anything is
            worse than no button, and this is the one place it would be seen. */}
        {items.length > 0 ? (
          <IconButton
            icon="delete_sweep"
            size={30}
            label="Empty the shelf"
            onClick={() => void clear(false)}
          />
        ) : null}
        <IconButton
          icon="close"
          size={30}
          label="Close"
          onClick={() => void actions.setState("shelfOpen", false)}
        />
      </header>

      {problem ? <p className="bw-shelf-problem">{problem}</p> : null}

      {items.length ? (
        <ul className="bw-shelf-items">
          {items.map((item) => (
            <li key={item.id}>
              <div
                className={[
                  "bw-shelf-item",
                  selected.includes(item.id) ? "selected" : "",
                  item.missing ? "missing" : "",
                ]
                  .filter(Boolean)
                  .join(" ")}
                onMouseDown={(event) => {
                  if (event.button !== 0) return;
                  toggleSelected(item.id, event.ctrlKey || event.shiftKey);
                  press.current = {
                    id: item.id,
                    x: event.clientX,
                    y: event.clientY,
                  };
                }}
                onMouseMove={(event) => {
                  const from = press.current;
                  if (!from || from.id !== item.id) return;
                  const moved =
                    Math.abs(event.clientX - from.x) +
                    Math.abs(event.clientY - from.y);
                  if (moved < DRAG_THRESHOLD) return;
                  // Handed over while the button is still down: the backend's
                  // drag takes the mouse from here.
                  press.current = null;
                  void startDrag(item.id);
                }}
                onMouseUp={() => {
                  press.current = null;
                }}
                onDoubleClick={() => void actions.openShelfItem(item.id)}
                title={item.path}
              >
                <Symbol
                  name={item.missing ? "link_off" : symbol(item.kind)}
                  size={22}
                  className="bw-shelf-glyph"
                />
                <span className="bw-shelf-text">
                  <span className="bw-shelf-name">{item.name}</span>
                  <span className="bw-shelf-detail">
                    {item.missing
                      ? tr("Not where it was")
                      : item.size === null
                        ? tr("Folder")
                        : readableSize(item.size)}
                  </span>
                </span>
                <span className="bw-shelf-actions">
                  <IconButton
                    icon="folder_open"
                    size={26}
                    label="Show in Explorer"
                    onClick={() => void actions.revealShelfItem(item.id)}
                  />
                  <IconButton
                    icon="close"
                    size={26}
                    label="Take off the shelf"
                    onClick={() => void remove(item.id)}
                  />
                </span>
              </div>
            </li>
          ))}
        </ul>
      ) : (
        <div className="bw-shelf-empty">
          <Symbol name="inbox" size={40} />
          <p>{tr("Drag files here to put them down for a moment")}</p>
          <p className="bw-shelf-note">
            {tr("The shelf remembers where they are, not a copy of them")}
          </p>
        </div>
      )}
    </div>
  );
}
