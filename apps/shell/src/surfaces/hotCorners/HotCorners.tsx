// The corners you throw the pointer at.
//
// This window covers the whole screen, but the backend has cut its window
// region down to the four strips before it was ever shown — everything outside
// them is not part of the window at all, and a click there reaches whatever is
// underneath. So the page can lay the strips out in absolute screen
// coordinates and rely on the region to keep it out of the way.
//
// The strips are invisible by default, which is the point and also the danger:
// a region a few pixels out is a sidebar that opens when somebody reaches for
// a window's close button. `sidebar.cornerOpen.visualize` paints them for
// exactly that reason.

import { useCallback, useEffect, useState } from "react";
import type { Corner, HotCorner, ScreenChrome } from "@bw/core";
import { Event } from "@bw/core";
import { connect, actions, useShell } from "../../shell/store";
import { backend } from "../../shell/backend";
import "./hotCorners.css";

export function HotCorners() {
  const ready = useShell((state) => state.ready);
  const config = useShell((state) => state.config.sidebar.cornerOpen);
  const [corners, setCorners] = useState<HotCorner[]>([]);
  const [active, setActive] = useState(true);

  useEffect(() => {
    void connect();
    void actions
      .hotCorners()
      .then(setCorners)
      .catch(() => setCorners([]));
    void actions
      .screenChrome()
      .then((chrome) => setActive(chrome.hotCornersActive))
      .catch(() => setActive(true));

    let stop: (() => void) | undefined;
    void backend()
      .listen<ScreenChrome>(Event.Chrome, (chrome) => {
        setActive(chrome.hotCornersActive);
        // The strips move with the config, and the config is what the chrome
        // event follows, so this is the moment to re-ask.
        void actions
          .hotCorners()
          .then(setCorners)
          .catch(() => setCorners([]));
      })
      .then((off) => {
        stop = off;
      });
    return () => stop?.();
  }, []);

  const fire = useCallback((corner: Corner) => {
    // Refusals are the backend's to report: a corner bound to a flag that no
    // longer exists is a config problem, and there is nowhere here to show it.
    void actions.runHotCorner(corner).catch(() => {});
  }, []);

  if (!ready || !active) return null;

  return (
    <div className="bw-hot-corners">
      {corners.map((hot) => (
        <div
          key={hot.corner}
          className={["bw-hot-corner", config.visualize ? "visible" : ""]
            .filter(Boolean)
            .join(" ")}
          style={{
            left: hot.rect.x,
            top: hot.rect.y,
            width: hot.rect.width,
            height: hot.rect.height,
          }}
          onMouseEnter={() => {
            if (config.clickless) fire(hot.corner);
          }}
          onMouseDown={(event) => {
            if (event.button !== 0) return;
            if (!config.clickless) fire(hot.corner);
          }}
          onWheel={(event) => {
            if (!config.valueScroll) return;
            void actions
              .scrollHotCorner(hot.corner, event.deltaY < 0)
              .catch(() => {});
          }}
        />
      ))}
    </div>
  );
}
