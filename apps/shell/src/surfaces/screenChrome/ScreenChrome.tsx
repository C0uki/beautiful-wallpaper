// The screen's own decorations: fake rounded corners, and a frame.
//
// Both are drawn on one window that covers the whole display and is
// click-through everywhere — there is nothing here to press. Four separate
// corner windows and four more frame windows is what this would be if it were
// a direct translation of the original, and eight webviews to paint eight
// coloured shapes is not a translation worth making.
//
// The corners are drawn as four boxes with one rounded outer edge each and the
// screen's background colour showing through the curve. That is the trick the
// original uses too: nothing is cut out of the screen, a shape is laid over
// each corner that happens to be the shape of the missing bit.

import { useEffect, useState } from "react";
import type { Edge, ScreenChrome as Chrome } from "@bw/core";
import { Event } from "@bw/core";
import { connect, actions, useShell } from "../../shell/store";
import { backend } from "../../shell/backend";
import "./screenChrome.css";

/** The colour a frame is painted in.
 *
 * A palette role name becomes the variable that holds it; anything else is
 * handed to CSS untouched, so `#101014` and `black` both work. */
function frameColor(name: string): string {
  const role = name.trim();
  if (!role) return "var(--scrim)";
  return /^[a-z][a-zA-Z0-9]*$/.test(role) && ROLES.has(role)
    ? `var(--${role})`
    : role;
}

/** The palette roles a frame may name. Anything else is a CSS colour. */
const ROLES = new Set([
  "primary",
  "secondary",
  "tertiary",
  "surface",
  "outline",
  "scrim",
  "shadow",
]);

export function ScreenChrome() {
  const ready = useShell((state) => state.ready);
  const [chrome, setChrome] = useState<Chrome | null>(null);

  useEffect(() => {
    void connect();
    void actions
      .screenChrome()
      .then(setChrome)
      .catch(() => setChrome(null));

    let stop: (() => void) | undefined;
    void backend()
      .listen<Chrome>(Event.Chrome, setChrome)
      .then((off) => {
        stop = off;
      });
    return () => stop?.();
  }, []);

  if (!ready || !chrome) return null;

  const thickness = chrome.frameThickness;
  const color = frameColor(chrome.frameColor);
  const has = (edge: Edge) => chrome.frameEdges.includes(edge);

  return (
    <div className="bw-chrome">
      {/* The frame, one strip per edge that asked for one. */}
      {has("top") ? (
        <div
          className="bw-chrome-frame"
          style={{
            top: 0,
            left: 0,
            right: 0,
            height: thickness,
            background: color,
          }}
        />
      ) : null}
      {has("bottom") ? (
        <div
          className="bw-chrome-frame"
          style={{
            bottom: 0,
            left: 0,
            right: 0,
            height: thickness,
            background: color,
          }}
        />
      ) : null}
      {has("left") ? (
        <div
          className="bw-chrome-frame"
          style={{
            top: 0,
            bottom: 0,
            left: 0,
            width: thickness,
            background: color,
          }}
        />
      ) : null}
      {has("right") ? (
        <div
          className="bw-chrome-frame"
          style={{
            top: 0,
            bottom: 0,
            right: 0,
            width: thickness,
            background: color,
          }}
        />
      ) : null}

      {/* The corners sit inside the frame when there is one, so the curve
          follows the frame's inner edge rather than being buried under it. */}
      {chrome.cornersVisible ? (
        <div
          className="bw-chrome-corners"
          style={{
            top: has("top") ? thickness : 0,
            bottom: has("bottom") ? thickness : 0,
            left: has("left") ? thickness : 0,
            right: has("right") ? thickness : 0,
          }}
        >
          {(["tl", "tr", "bl", "br"] as const).map((corner) => (
            <span
              key={corner}
              className={`bw-chrome-corner ${corner}`}
              style={{
                width: chrome.radius,
                height: chrome.radius,
                // The curve is the *inside* of the shape, so the radius goes
                // on the corner facing the middle of the screen.
                borderRadius: cornerRadius(corner, chrome.radius),
              }}
            />
          ))}
        </div>
      ) : null}
    </div>
  );
}

/** Rounds only the edge that faces the middle of the screen. */
function cornerRadius(
  corner: "tl" | "tr" | "bl" | "br",
  radius: number,
): string {
  const r = `${radius}px`;
  switch (corner) {
    case "tl":
      return `0 0 ${r} 0`;
    case "tr":
      return `0 0 0 ${r}`;
    case "bl":
      return `0 ${r} 0 0`;
    case "br":
      return `${r} 0 0 0`;
  }
}
