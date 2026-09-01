// The floating overlay: the half that never takes the pointer.
//
// A separate window because a window region masks drawing and hit-testing
// together, and everything here has to be drawn without being clickable — a
// pinned crosshair that swallowed clicks in the middle of the screen would be
// unusable, which is the whole reason it is pinned in the first place.
//
// There is no chrome and nothing to press. The backend makes this window
// click-through once, when it is created.

import { useEffect, useState } from "react";
import type { Crosshair, OverlayLayout } from "@bw/core";
import { Event } from "@bw/core";
import { actions, connect, useShell } from "../../shell/store";
import { backend } from "../../shell/backend";
import { Canvas } from "./Canvas";
import "./overlay.css";

export function OverlayPinned() {
  const ready = useShell((state) => state.ready);
  const opacity = useShell((state) => state.config.overlay.clickthroughOpacity);
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

  if (!ready || !layout || !layout.passiveVisible) return null;

  return (
    <div className="bw-overlay passive">
      <Canvas
        placed={layout.passive}
        crosshair={crosshair}
        chrome={false}
        clickthroughOpacity={opacity}
      />
    </div>
  );
}
