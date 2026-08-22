// The desktop widget canvas.
//
// A port of end4-pC's `modules/common/widgets/widgetCanvas`: widgets are placed
// freely, dragged in an edit mode, and snapped to a grid on release.
//
// Positions are stored as fractions of the monitor, not pixels, so a resolution
// change or a move to a different monitor keeps the layout recognisable instead
// of pushing widgets off-screen.

import {
  useCallback,
  useEffect,
  useRef,
  useState,
  type ReactNode,
} from "react";
import type { WidgetPlacement } from "@bw/core";
import { actions } from "../../shell/store";
import { Symbol } from "../../widgets";

export interface CanvasItem {
  id: string;
  placement: WidgetPlacement;
  /** Dotted config path of this widget's placement, for persisting a drag. */
  configPath: string;
  node: ReactNode;
}

export interface WidgetCanvasProps {
  items: CanvasItem[];
  editing: boolean;
  /** Snap step in pixels; 0 disables snapping. */
  grid: number;
}

interface DragState {
  id: string;
  configPath: string;
  pointerId: number;
  /** Offset from the widget's top-left to the pointer, in pixels. */
  offsetX: number;
  offsetY: number;
}

export function WidgetCanvas({ items, editing, grid }: WidgetCanvasProps) {
  const rootRef = useRef<HTMLDivElement | null>(null);
  const dragRef = useRef<DragState | null>(null);
  // Position overrides while dragging, so the widget tracks the pointer without
  // a round trip through the backend on every frame.
  const [live, setLive] = useState<Record<string, { x: number; y: number }>>(
    {},
  );

  const snap = useCallback(
    (pixels: number) => (grid > 0 ? Math.round(pixels / grid) * grid : pixels),
    [grid],
  );

  const onPointerDown = useCallback(
    (event: React.PointerEvent<HTMLDivElement>, item: CanvasItem) => {
      if (!editing) return;
      const element = event.currentTarget;
      const box = element.getBoundingClientRect();
      element.setPointerCapture(event.pointerId);
      dragRef.current = {
        id: item.id,
        configPath: item.configPath,
        pointerId: event.pointerId,
        offsetX: event.clientX - box.left,
        offsetY: event.clientY - box.top,
      };
      event.preventDefault();
    },
    [editing],
  );

  const onPointerMove = useCallback(
    (event: React.PointerEvent<HTMLDivElement>) => {
      const drag = dragRef.current;
      const root = rootRef.current;
      if (!drag || !root || drag.pointerId !== event.pointerId) return;

      const bounds = root.getBoundingClientRect();
      const element = event.currentTarget;
      const size = element.getBoundingClientRect();

      // Keep the whole widget on screen, whichever edge it was dragged towards.
      const maxX = Math.max(0, bounds.width - size.width);
      const maxY = Math.max(0, bounds.height - size.height);
      const x = Math.min(
        maxX,
        Math.max(0, snap(event.clientX - bounds.left - drag.offsetX)),
      );
      const y = Math.min(
        maxY,
        Math.max(0, snap(event.clientY - bounds.top - drag.offsetY)),
      );

      setLive((current) => ({
        ...current,
        [drag.id]: { x: x / bounds.width, y: y / bounds.height },
      }));
    },
    [snap],
  );

  const endDrag = useCallback((event: React.PointerEvent<HTMLDivElement>) => {
    const drag = dragRef.current;
    if (!drag || drag.pointerId !== event.pointerId) return;
    dragRef.current = null;

    setLive((current) => {
      const position = current[drag.id];
      if (position) {
        // Persist once, on release — not on every pointer move.
        void actions.setConfigValue(`${drag.configPath}.x`, position.x);
        void actions.setConfigValue(`${drag.configPath}.y`, position.y);
      }
      return current;
    });
  }, []);

  // Leaving edit mode drops the local overrides; the config is authoritative again.
  useEffect(() => {
    if (!editing) setLive({});
  }, [editing]);

  return (
    <div
      ref={rootRef}
      style={{
        position: "absolute",
        inset: 0,
        pointerEvents: editing ? "auto" : "none",
      }}
    >
      {items
        .filter((item) => item.placement.enable)
        .map((item) => {
          const position = live[item.id] ?? {
            x: item.placement.x,
            y: item.placement.y,
          };
          return (
            <div
              key={item.id}
              onPointerDown={(event) => onPointerDown(event, item)}
              onPointerMove={onPointerMove}
              onPointerUp={endDrag}
              onPointerCancel={endDrag}
              style={{
                position: "absolute",
                left: `${position.x * 100}%`,
                top: `${position.y * 100}%`,
                // Widgets themselves stay interactive outside edit mode (media
                // buttons, for instance); only the canvas is click-through.
                pointerEvents: "auto",
                cursor: editing ? "grab" : "default",
                touchAction: "none",
                transition: dragRef.current
                  ? "none"
                  : "left var(--duration-slow) var(--ease-spatial-default), top var(--duration-slow) var(--ease-spatial-default)",
              }}
            >
              {editing ? (
                <div
                  style={{
                    position: "absolute",
                    inset: -6,
                    borderRadius: 22,
                    border: "2px dashed var(--primary)",
                    pointerEvents: "none",
                  }}
                >
                  <Symbol
                    name="drag_indicator"
                    size={18}
                    color="var(--primary)"
                    style={{ position: "absolute", top: -10, left: -10 }}
                  />
                </div>
              ) : null}
              {item.node}
            </div>
          );
        })}
    </div>
  );
}
