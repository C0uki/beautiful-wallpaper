// The volume and brightness readout.
//
// A pill with the level in it, shown while the value is changing and gone a
// second later. The backend owns the timing — it knows when the last change
// arrived, and a burst of them (holding a volume key) should keep the readout
// up rather than letting an early timer close it mid-press.

import { useEffect, useState } from "react";
import { Symbol } from "../../widgets";
import { backend } from "../../shell/backend";
import { Event } from "@bw/core";
import "./osd.css";

/** What the backend asks the readout to show. */
interface OsdEvent {
  kind: "volume" | "brightness";
  value: number;
  muted: boolean;
}

/** Volume has three icons; which one depends on how loud it is. */
function volumeIcon(percent: number, muted: boolean): string {
  if (muted || percent <= 0) return "volume_off";
  if (percent < 50) return "volume_down";
  return "volume_up";
}

function brightnessIcon(percent: number): string {
  return percent < 40 ? "brightness_low" : "brightness_high";
}

export function Osd() {
  const [reading, setReading] = useState<OsdEvent | null>(null);

  useEffect(() => {
    let cancelled = false;
    const subscription = backend().listen<OsdEvent>(Event.Osd, (next) => {
      if (!cancelled) setReading(next);
    });
    return () => {
      cancelled = true;
      void subscription.then((unlisten) => unlisten());
    };
  }, []);

  if (!reading) return null;

  const muted = reading.kind === "volume" && reading.muted;
  const icon =
    reading.kind === "volume"
      ? volumeIcon(reading.value, reading.muted)
      : brightnessIcon(reading.value);
  // A muted readout still shows where the level would be, greyed, so unmuting
  // is not a surprise.
  const percent = Math.min(Math.max(reading.value, 0), 100);

  return (
    <div className="bw-osd" data-muted={muted}>
      <div className="bw-osd-icon">
        <Symbol name={icon} size={22} filled />
      </div>
      <div className="bw-osd-track">
        <div className="bw-osd-fill" style={{ width: `${percent}%` }} />
      </div>
      <span className="bw-osd-value">{Math.round(percent)}</span>
    </div>
  );
}
