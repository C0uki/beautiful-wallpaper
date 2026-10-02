// A spectrum of whatever the speakers are playing.
//
// The capture runs in the shell only while one of these is on screen: each
// one asks for it when it mounts and lets go when it unmounts, and the shell
// stops listening once nobody is asking.

import { useEffect, useState } from "react";
import { Command, Event } from "@bw/core";
import { backend } from "../shell/backend";

export function Visualizer({
  bars,
  className,
}: {
  /** How many bars to draw, taken evenly from what the shell sends. */
  bars: number;
  className?: string;
}) {
  const [levels, setLevels] = useState<number[]>([]);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let gone = false;
    void backend()
      .listen<number[]>(Event.Visualizer, setLevels)
      .then((stop) => {
        if (gone) stop();
        else unlisten = stop;
      });
    void backend().invoke<void>(Command.WatchVisualizer, { on: true });
    return () => {
      gone = true;
      unlisten?.();
      void backend().invoke<void>(Command.WatchVisualizer, { on: false });
    };
  }, []);

  return (
    <div className={["bw-visualizer", className].filter(Boolean).join(" ")}>
      {Array.from({ length: bars }, (_, index) => {
        const level = levels[Math.floor((index * levels.length) / bars)] ?? 0;
        return (
          <span
            key={index}
            // A sliver even when silent, so the shape is there to grow from.
            style={{ transform: `scaleY(${Math.max(0.06, level)})` }}
          />
        );
      })}
    </div>
  );
}
