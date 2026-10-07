// The desktop widget canvas.
//
// A port of end4-pC's `modules/common/widgets/widgetCanvas`: widgets are placed
// freely, dragged in an edit mode, and snapped to a grid on release.
//
// Positions are stored as fractions of the monitor, not pixels, so a resolution
// change or a move to a different monitor keeps the layout recognisable instead
// of pushing widgets off-screen.
//
// A widget whose `placementStrategy` is `"leastBusy"` ignores its stored
// position and goes wherever the wallpaper is calmest, clear of the others.
// Dragging one is choosing a place for it, so it becomes `"free"` there.

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
import { busyness, calmestSpot, type Box } from "./calm";

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
  /** The wallpaper on screen, for the widgets placed where it is calmest. */
  wallpaper?: string;
  /** How much larger than the screen the wallpaper is drawn, for parallax. */
  zoom?: number;
}

type Spot = { x: number; y: number };

const isCalm = (item: CanvasItem) =>
  item.placement.enable && item.placement.placementStrategy === "leastBusy";

/**
 * Where each `leastBusy` widget goes on `image`, as fractions of the screen.
 *
 * The wallpaper is read at 160 pixels across, framed the way it is drawn
 * ("cover", then enlarged by the parallax `zoom` about the middle), and each
 * widget takes the calmest free place in turn, around the ones that stay
 * where they were put. The parallax pan follows the pointer around the
 * middle, so the picture is read where it rests: centred.
 */
function placeCalmly(
  image: HTMLImageElement,
  root: HTMLElement,
  items: CanvasItem[],
  elements: Map<string, HTMLElement>,
  zoom: number,
): Record<string, Spot> {
  const bounds = root.getBoundingClientRect();
  if (!bounds.width || !bounds.height) return {};
  const width = 160;
  const height = Math.max(
    1,
    Math.round((width * bounds.height) / bounds.width),
  );
  const canvas = document.createElement("canvas");
  canvas.width = width;
  canvas.height = height;
  const context = canvas.getContext("2d", { willReadFrequently: true });
  if (!context) return {};
  const scale =
    Math.max(
      width / (image.naturalWidth || 1),
      height / (image.naturalHeight || 1),
    ) * zoom;
  const drawnWidth = image.naturalWidth * scale;
  const drawnHeight = image.naturalHeight * scale;
  context.drawImage(
    image,
    (width - drawnWidth) / 2,
    (height - drawnHeight) / 2,
    drawnWidth,
    drawnHeight,
  );
  const { data } = context.getImageData(0, 0, width, height);
  const luma = new Float32Array(width * height);
  for (let i = 0; i < luma.length; i++) {
    luma[i] =
      0.299 * data[i * 4]! +
      0.587 * data[i * 4 + 1]! +
      0.114 * data[i * 4 + 2]!;
  }
  const busy = busyness(luma, width, height);

  const toMap = width / bounds.width;
  const sizeOf = (id: string) => {
    const box = elements.get(id)?.getBoundingClientRect();
    return box
      ? {
          width: Math.ceil(box.width * toMap),
          height: Math.ceil(box.height * toMap),
        }
      : null;
  };

  const taken: Box[] = [];
  for (const item of items) {
    if (!item.placement.enable || isCalm(item)) continue;
    const size = sizeOf(item.id);
    if (size) {
      taken.push({
        x: item.placement.x * width,
        y: item.placement.y * height,
        ...size,
      });
    }
  }

  const placed: Record<string, Spot> = {};
  for (const item of items.filter(isCalm)) {
    const size = sizeOf(item.id);
    if (!size) continue;
    const spot = calmestSpot(
      busy,
      width,
      height,
      size.width,
      size.height,
      taken,
      4,
    );
    taken.push({ ...spot, ...size });
    placed[item.id] = { x: spot.x / width, y: spot.y / height };
  }
  return placed;
}

interface DragState {
  id: string;
  configPath: string;
  pointerId: number;
  /** Offset from the widget's top-left to the pointer, in pixels. */
  offsetX: number;
  offsetY: number;
  /** Placed by the wallpaper until now; the drop makes it `"free"`. */
  calm: boolean;
}

export function WidgetCanvas({
  items,
  editing,
  grid,
  wallpaper,
  zoom = 1,
}: WidgetCanvasProps) {
  const rootRef = useRef<HTMLDivElement | null>(null);
  const dragRef = useRef<DragState | null>(null);
  // Position overrides while dragging, so the widget tracks the pointer without
  // a round trip through the backend on every frame.
  const [live, setLive] = useState<Record<string, { x: number; y: number }>>(
    {},
  );
  const elements = useRef(new Map<string, HTMLElement>());
  const [calm, setCalm] = useState<Record<string, Spot>>({});

  // Everything the calm places depend on, as one value the effect can watch:
  // which widgets are placed by the wallpaper, and where the others stand.
  const itemsRef = useRef(items);
  itemsRef.current = items;
  const layout = items
    .map(
      ({ id, placement }) =>
        `${id}:${placement.enable}:${placement.placementStrategy}:${placement.x}:${placement.y}`,
    )
    .join("|");

  useEffect(() => {
    const root = rootRef.current;
    if (!wallpaper || !root || !itemsRef.current.some(isCalm)) {
      setCalm({});
      return;
    }
    let cancelled = false;
    const image = new Image();
    image.crossOrigin = "anonymous";
    image.onload = () => {
      if (!cancelled) {
        setCalm(
          placeCalmly(image, root, itemsRef.current, elements.current, zoom),
        );
      }
    };
    image.src = wallpaper;
    return () => {
      cancelled = true;
    };
  }, [wallpaper, layout, zoom]);

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
        calm: isCalm(item),
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
        if (drag.calm) {
          void actions.setConfigValue(
            `${drag.configPath}.placementStrategy`,
            "free",
          );
        }
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
          const position = live[item.id] ??
            (isCalm(item) ? calm[item.id] : undefined) ?? {
              x: item.placement.x,
              y: item.placement.y,
            };
          return (
            <div
              key={item.id}
              ref={(element) => {
                if (element) elements.current.set(item.id, element);
                else elements.current.delete(item.id);
              }}
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
