// A popup that appears under a bar widget on hover.
//
// end4-pC's bar widgets each carry one of these for their detail view. The delay
// matters: without it, sweeping the pointer across the bar flashes every popup
// in turn.

import { useEffect, useRef, useState, type ReactNode } from "react";
import { shape } from "@bw/tokens";

export interface HoverPopupProps {
  content: ReactNode;
  children: ReactNode;
  /** Milliseconds to wait before showing. */
  delay?: number;
}

export function HoverPopup({
  content,
  children,
  delay = shape.sizes.hoverPopupDelay,
}: HoverPopupProps) {
  const [open, setOpen] = useState(false);
  const timer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);

  useEffect(() => () => clearTimeout(timer.current), []);

  const show = () => {
    clearTimeout(timer.current);
    timer.current = setTimeout(() => setOpen(true), delay);
  };
  const hide = () => {
    clearTimeout(timer.current);
    setOpen(false);
  };

  return (
    <div
      onPointerEnter={show}
      onPointerLeave={hide}
      style={{
        position: "relative",
        display: "flex",
        alignItems: "center",
        height: "100%",
      }}
    >
      {children}
      {open ? (
        <div
          className="bw-card"
          data-elevation="2"
          style={{
            position: "absolute",
            top: "calc(100% + 6px)",
            left: "50%",
            transform: "translateX(-50%)",
            zIndex: 30,
            padding: "10px 12px",
            fontSize: "0.85em",
            pointerEvents: "none",
            animation:
              "bw-popup-in var(--duration-fast) var(--ease-spatial-default)",
          }}
        >
          {content}
        </div>
      ) : null}
    </div>
  );
}
