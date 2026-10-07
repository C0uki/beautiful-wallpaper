// The screen's own decoration: a frame.
//
// Drawn on one window that covers the whole display and is click-through
// everywhere — there is nothing here to press. Four frame windows is what this
// would be as a direct translation of the original, and four webviews to paint
// four coloured strips is not a translation worth making.

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
    </div>
  );
}
