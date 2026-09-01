// The floating overlay: the half that takes the pointer.
//
// While the overlay is open this is the whole screen — the backdrop included,
// because clicking past the widgets is how it is dismissed. Once it is shut,
// the backend cuts this window's region down to the pinned widgets that still
// want clicks, and everything else on the desktop carries on underneath.
//
// So the same component draws two quite different things, and the difference
// is `open`: with it, a scrim and a taskbar and draggable widgets; without it,
// bare widgets on nothing at all.

import { useCallback, useEffect, useState } from "react";
import type { Crosshair, OverlayLayout, OverlayWidget } from "@bw/core";
import { Event } from "@bw/core";
import { Symbol } from "../../widgets";
import { tr } from "../../i18n";
import { actions, connect, useShell } from "../../shell/store";
import { backend } from "../../shell/backend";
import { Canvas, widgetTitle } from "./Canvas";
import "./overlay.css";

/** Every widget the taskbar offers, in a fixed order. */
const CATALOGUE: { widget: OverlayWidget; symbol: string }[] = [
  { widget: "crosshair", symbol: "point_scan" },
  { widget: "notes", symbol: "note_stack" },
  { widget: "resources", symbol: "browse_activity" },
];

export function Overlay() {
  const ready = useShell((state) => state.ready);
  const open = useShell((state) => state.states.overlayOpen);
  const opacity = useShell((state) => state.config.overlay.clickthroughOpacity);
  const placedNow = useShell((state) => state.persistent.overlay.open);

  const [layout, setLayout] = useState<OverlayLayout | null>(null);
  const [crosshair, setCrosshair] = useState<Crosshair | null>(null);

  useEffect(() => {
    void connect();
    void actions
      .overlayLayout()
      .then(setLayout)
      .catch(() => setLayout(null));
    void actions
      .crosshair()
      .then(setCrosshair)
      .catch(() => setCrosshair(null));

    const stop: Array<() => void> = [];
    const api = backend();
    void api
      .listen<OverlayLayout>(Event.Overlay, setLayout)
      .then((off) => stop.push(off));
    // The code lives in the config, so a reload can change the crosshair
    // without anything about the layout moving.
    void api
      .listen(Event.ConfigChanged, () => {
        void actions
          .crosshair()
          .then(setCrosshair)
          .catch(() => {});
      })
      .then((off) => stop.push(off));

    return () => {
      for (const off of stop) off();
    };
  }, []);

  const close = useCallback(
    () => void actions.setState("overlayOpen", false),
    [],
  );

  useEffect(() => {
    if (!open) return;
    const onKey = (event: KeyboardEvent) => {
      // Not while typing into a note: Escape there should not take the whole
      // overlay away.
      if (event.key !== "Escape") return;
      if (document.activeElement instanceof HTMLTextAreaElement) return;
      close();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [close, open]);

  if (!ready || !layout) return null;
  if (!open && !layout.interactiveVisible) return null;

  return (
    <div
      className={["bw-overlay", open ? "open" : "pinned"].join(" ")}
      // Autofocus while open so Escape and the arrow keys land here.
      ref={(element) => {
        if (open) element?.focus();
      }}
      tabIndex={open ? -1 : undefined}
      onMouseDown={(event) => {
        // Only while open: when shut, this element only exists where the
        // window's region says it does, and every pixel of it is a widget.
        if (open && event.target === event.currentTarget) close();
      }}
    >
      {layout.scrim && open ? <div className="bw-overlay-scrim" /> : null}

      {open ? (
        <nav className="bw-overlay-taskbar" aria-label={tr("Overlay widgets")}>
          {CATALOGUE.map((entry) => {
            const on = placedNow.includes(entry.widget);
            return (
              <button
                key={entry.widget}
                type="button"
                className={on ? "on" : ""}
                aria-pressed={on}
                onClick={() => void actions.toggleOverlayWidget(entry.widget)}
              >
                <Symbol name={entry.symbol} size={22} />
                <span>{widgetTitle(entry.widget)}</span>
              </button>
            );
          })}
        </nav>
      ) : null}

      <Canvas
        placed={layout.interactive}
        crosshair={crosshair}
        chrome={open}
        clickthroughOpacity={opacity}
      />
    </div>
  );
}
