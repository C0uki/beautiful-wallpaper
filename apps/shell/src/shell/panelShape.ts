// The desktop blurred behind a panel on a full-screen surface.
//
// Windows blurs a whole window, and the search, the menus and the settings
// each cover the screen with one that is mostly transparent sheet. So while
// the panel is up its window is cut down to the panel's outline and blurred
// (`set_panel_shape`), and the panel thins its own fill to let the blur
// through. Away, the window collapses to nothing, so it cannot flash a
// full-screen blur on its way back before its outline is known.

import { useEffect, useState } from "react";
import { actions, useShell } from "./store";

/** Where an element sits once laid out, in CSS pixels — not where an
 *  opening animation's transform has it this frame, which a measurement
 *  taken as the panel appears would otherwise freeze into its outline. */
function layoutBox(element: HTMLElement) {
  let x = 0;
  let y = 0;
  for (
    let node: HTMLElement | null = element;
    node;
    node = node.offsetParent as HTMLElement | null
  ) {
    x += node.offsetLeft;
    y += node.offsetTop;
  }
  return { x, y, width: element.offsetWidth, height: element.offsetHeight };
}

/** A ref for the panel; while `active` is truthy, its window is cut to it
 *  and blurred. A new `active` measures again, for a panel that moves
 *  without changing size. */
export function usePanelShape<T extends HTMLElement>(active: unknown) {
  const [element, setElement] = useState<T | null>(null);
  const backdrop = useShell((state) => state.config.windows.backdrop);

  useEffect(() => {
    if (backdrop === "none") {
      // The whole window back, unblurred, in case it was collapsed while
      // the blur was on.
      void actions.setPanelShape(0, 0, -1, 0, 0);
      return;
    }
    if (!active || !element) return;

    const send = () => {
      const box = layoutBox(element);
      const radius = parseFloat(getComputedStyle(element).borderTopLeftRadius);
      void actions.setPanelShape(
        box.x,
        box.y,
        box.width,
        box.height,
        Number.isFinite(radius) ? radius : 0,
      );
    };
    element.dataset.blurred = "true";
    send();
    const observer = new ResizeObserver(send);
    observer.observe(element);
    window.addEventListener("resize", send);
    return () => {
      observer.disconnect();
      window.removeEventListener("resize", send);
      delete element.dataset.blurred;
      void actions.setPanelShape(0, 0, 0, 0, 0);
    };
  }, [active, element, backdrop]);

  return setElement;
}
