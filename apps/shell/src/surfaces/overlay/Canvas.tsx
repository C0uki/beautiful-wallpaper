// The overlay's widgets, drawn where the layout says.
//
// Used by both of the overlay's windows. The interactive one passes
// `chrome`, which adds the title bar with the pin, see-through and close
// buttons and makes the widget draggable; the passive one does not, and gets
// bare widgets with nothing to press.
//
// Positions are CSS pixels, because that is what the page is measured in. The
// backend converts to real device pixels in the one place that needs them —
// the window region — rather than making every position carry the difference.

import { useCallback, useEffect, useRef, useState } from "react";
import type { Crosshair, OverlayWidget, Placed } from "@bw/core";
import { IconButton } from "../../widgets";
import { tr } from "../../i18n";
import { actions } from "../../shell/store";
import { CrosshairWidget } from "./widgets/CrosshairWidget";
import { NotesWidget } from "./widgets/NotesWidget";
import { ResourcesWidget } from "./widgets/ResourcesWidget";

/** What each widget is called, in the user's language. */
export function widgetTitle(widget: OverlayWidget): string {
  switch (widget) {
    case "crosshair":
      return tr("Crosshair");
    case "notes":
      return tr("Notes");
    case "resources":
      return tr("Resources");
  }
}

interface CanvasProps {
  placed: Placed[];
  crosshair: Crosshair | null;
  /** Whether to draw the title bars and allow dragging. */
  chrome: boolean;
  /** How solid a see-through widget looks. */
  clickthroughOpacity: number;
}

export function Canvas({
  placed,
  crosshair,
  chrome,
  clickthroughOpacity,
}: CanvasProps) {
  return (
    <>
      {placed.map((widget) => (
        <Frame
          key={widget.widget}
          placed={widget}
          crosshair={crosshair}
          chrome={chrome}
          clickthroughOpacity={clickthroughOpacity}
        />
      ))}
    </>
  );
}

function Frame({
  placed,
  crosshair,
  chrome,
  clickthroughOpacity,
}: {
  placed: Placed;
  crosshair: Crosshair | null;
  chrome: boolean;
  clickthroughOpacity: number;
}) {
  const [drag, setDrag] = useState<{ x: number; y: number } | null>(null);
  const from = useRef<{
    pointer: [number, number];
    at: [number, number];
  } | null>(null);

  // A drag is followed on the window rather than on the header: the pointer
  // routinely leaves a small box mid-drag, and a handler on the element itself
  // would drop the widget the moment it did.
  useEffect(() => {
    if (!drag) return;

    const move = (event: MouseEvent) => {
      const start = from.current;
      if (!start) return;
      setDrag({
        x: start.at[0] + (event.clientX - start.pointer[0]),
        y: start.at[1] + (event.clientY - start.pointer[1]),
      });
    };
    const up = () => {
      const settled = drag;
      from.current = null;
      setDrag(null);
      // Persisted on release, not per pixel: this goes to disk, and the
      // backend re-cuts the window's region on every write.
      void actions.setPersistentValue(
        `overlay.${placed.widget}.x`,
        Math.round(settled.x),
      );
      void actions.setPersistentValue(
        `overlay.${placed.widget}.y`,
        Math.round(settled.y),
      );
    };

    window.addEventListener("mousemove", move);
    window.addEventListener("mouseup", up);
    return () => {
      window.removeEventListener("mousemove", move);
      window.removeEventListener("mouseup", up);
    };
  }, [drag, placed.widget]);

  const startDrag = useCallback(
    (event: React.MouseEvent) => {
      if (event.button !== 0) return;
      event.preventDefault();
      from.current = {
        pointer: [event.clientX, event.clientY],
        at: [placed.rect.x, placed.rect.y],
      };
      setDrag({ x: placed.rect.x, y: placed.rect.y });
    },
    [placed.rect.x, placed.rect.y],
  );

  const set = (field: "pinned" | "clickthrough", value: boolean) =>
    void actions.setPersistentValue(`overlay.${placed.widget}.${field}`, value);

  const at = drag ?? { x: placed.rect.x, y: placed.rect.y };

  return (
    <div
      className={[
        "bw-overlay-widget",
        chrome ? "framed" : "",
        placed.widget === "crosshair" ? "bare" : "",
      ]
        .filter(Boolean)
        .join(" ")}
      style={{
        left: at.x,
        top: at.y,
        width: placed.rect.width,
        height: placed.rect.height,
        // Only meaningful once the overlay is shut and the widget is really
        // seeing clicks through it; while it is open everything is solid.
        opacity: !chrome && placed.clickthrough ? clickthroughOpacity : 1,
      }}
    >
      {chrome ? (
        <header className="bw-overlay-widget-head" onMouseDown={startDrag}>
          <span>{widgetTitle(placed.widget)}</span>
          <IconButton
            icon={placed.pinned ? "keep" : "keep_off"}
            size={26}
            label={placed.pinned ? "Unpin" : "Keep on screen"}
            onClick={() => set("pinned", !placed.pinned)}
          />
          <IconButton
            icon={placed.clickthrough ? "do_not_touch" : "touch_app"}
            size={26}
            label={
              placed.clickthrough ? "Make it clickable" : "Let clicks through"
            }
            onClick={() => set("clickthrough", !placed.clickthrough)}
          />
          <IconButton
            icon="close"
            size={26}
            label="Take it off the canvas"
            onClick={() => void actions.toggleOverlayWidget(placed.widget)}
          />
        </header>
      ) : null}

      <div className="bw-overlay-widget-body">
        <Content
          widget={placed.widget}
          crosshair={crosshair}
          interactive={chrome || !placed.clickthrough}
        />
      </div>
    </div>
  );
}

function Content({
  widget,
  crosshair,
  interactive,
}: {
  widget: OverlayWidget;
  crosshair: Crosshair | null;
  interactive: boolean;
}) {
  switch (widget) {
    case "crosshair":
      return crosshair ? <CrosshairWidget crosshair={crosshair} /> : null;
    case "notes":
      return <NotesWidget editable={interactive} />;
    case "resources":
      return <ResourcesWidget interactive={interactive} />;
  }
}
