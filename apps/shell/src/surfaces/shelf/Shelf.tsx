// The drop shelf.
//
// Files come in from anywhere on the machine and go out to anywhere else. Both
// halves are Windows' drag-and-drop rather than the browser's: on Windows the
// shell drop target the webview registers takes the drop before the page sees
// it, so `ondrop` never fires and the paths arrive as `tauri://drag-drop`
// instead. The HTML5 handlers below are for the development harness, where
// there is no such target and a real file dragged into the browser is the only
// way to test this at all.
//
// Dragging back out cannot be started by the page either — an application
// expecting a file wants shell items, not a web drag — so a press and a few
// pixels of movement hand the selection to the backend, which runs the real
// thing.

import { useCallback, useEffect, useRef, useState } from "react";
import type {
  DragDropPayload,
  DropOutcome,
  ShelfItem,
  ShelfKind,
} from "@bw/core";
import { DragEvent as Drag, Event } from "@bw/core";
import { IconButton, Symbol } from "../../widgets";
import { tr } from "../../i18n";
import { actions, connect, useShell } from "../../shell/store";
import { describeError } from "../../shell/errors";
import { backend } from "../../shell/backend";
import "./shelf.css";

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
    // Windows' own drag, which is the only one that carries real paths.
    void api
      .listen<DragDropPayload>(Drag.Enter, () => setHovering(true))
      .then((off) => stop.push(off));
    void api
      .listen<DragDropPayload>(Drag.Leave, () => setHovering(false))
      .then((off) => stop.push(off));
    void api
      .listen<DragDropPayload>(Drag.Drop, (payload) => {
        setHovering(false);
        void receive(payload.paths ?? []);
      })
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
      // Only ever reached in the harness: on Windows the shell drop target
      // above the page has already taken the drop.
      onDragOver={(event) => {
        event.preventDefault();
        setHovering(true);
      }}
      onDragLeave={() => setHovering(false)}
      onDrop={(event) => {
        event.preventDefault();
        setHovering(false);
        void receive(
          Array.from(event.dataTransfer.files).map((file) => file.name),
        );
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
