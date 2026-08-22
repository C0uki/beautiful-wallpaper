// The desktop clock.
//
// end4-pC ships several styles; the two ported here are the ones the screenshots
// lead with: a large "cookie" analogue face with a scalloped edge, and a plain
// digital readout.

import { useEffect, useState } from "react";
import { tr } from "../../../i18n";

function useNow(): Date {
  const [now, setNow] = useState(() => new Date());
  useEffect(() => {
    // Tick on the second boundary rather than every 1000 ms, so the minute
    // never appears to change late.
    let timer: ReturnType<typeof setTimeout>;
    const schedule = () => {
      const delay = 1000 - (Date.now() % 1000);
      timer = setTimeout(() => {
        setNow(new Date());
        schedule();
      }, delay);
    };
    schedule();
    return () => clearTimeout(timer);
  }, []);
  return now;
}

/** A scalloped disc, the shape Material calls a "cookie". */
function cookiePath(radius: number, lobes = 12, depth = 0.055): string {
  const points: string[] = [];
  const steps = lobes * 12;
  for (let index = 0; index <= steps; index += 1) {
    const angle = (index / steps) * Math.PI * 2;
    const wobble = 1 + depth * Math.cos(angle * lobes);
    const x = Math.cos(angle) * radius * wobble;
    const y = Math.sin(angle) * radius * wobble;
    points.push(`${index === 0 ? "M" : "L"}${x.toFixed(2)},${y.toFixed(2)}`);
  }
  return `${points.join("")}Z`;
}

export function ClockWidget({ style = "cookie" }: { style?: string }) {
  const now = useNow();

  if (style === "digital") {
    return (
      <div style={{ lineHeight: 1 }}>
        <div
          style={{
            fontFamily: "var(--font-title)",
            fontSize: "4.4em",
            fontWeight: 300,
            letterSpacing: "-0.02em",
            color: "var(--on-surface)",
            textShadow: "0 2px 18px var(--shadow)",
          }}
        >
          {now.getHours().toString().padStart(2, "0")}
          <span style={{ opacity: 0.5 }}>:</span>
          {now.getMinutes().toString().padStart(2, "0")}
        </div>
        <div
          style={{
            marginTop: 6,
            color: "var(--on-surface-variant)",
            fontSize: "1.05em",
          }}
        >
          {now.toLocaleDateString(undefined, {
            weekday: "long",
            day: "numeric",
            month: "long",
          })}
        </div>
      </div>
    );
  }

  const size = 200;
  const radius = size / 2 - 12;
  const seconds = now.getSeconds();
  const minutes = now.getMinutes() + seconds / 60;
  const hours = (now.getHours() % 12) + minutes / 60;

  const hand = (
    turns: number,
    length: number,
    width: number,
    color: string,
  ) => {
    const angle = turns * Math.PI * 2 - Math.PI / 2;
    return (
      <line
        x1={0}
        y1={0}
        x2={Math.cos(angle) * length}
        y2={Math.sin(angle) * length}
        stroke={color}
        strokeWidth={width}
        strokeLinecap="round"
      />
    );
  };

  return (
    <div style={{ position: "relative", width: size, height: size }}>
      <svg
        width={size}
        height={size}
        viewBox={`${-size / 2} ${-size / 2} ${size} ${size}`}
      >
        <path d={cookiePath(radius)} fill="var(--m3-primary-container)" />
        {[12, 3, 6, 9].map((hour, index) => {
          const angle = (index / 4) * Math.PI * 2 - Math.PI / 2;
          return (
            <text
              key={hour}
              x={Math.cos(angle) * radius * 0.66}
              y={Math.sin(angle) * radius * 0.66}
              textAnchor="middle"
              dominantBaseline="central"
              fontFamily="var(--font-title)"
              fontSize={radius * 0.42}
              fontWeight={500}
              fill="var(--m3-on-primary-container)"
              opacity={0.3}
            >
              {hour}
            </text>
          );
        })}
        {hand(hours / 12, radius * 0.46, 9, "var(--m3-on-primary-container)")}
        {hand(minutes / 60, radius * 0.74, 6, "var(--primary)")}
        <circle r={5.5} fill="var(--primary)" />
      </svg>

      <div
        style={{
          position: "absolute",
          top: -6,
          left: -6,
          minWidth: 46,
          padding: "6px 10px",
          borderRadius: 999,
          background: "var(--tertiary)",
          color: "var(--tertiary-on)",
          fontFamily: "var(--font-title)",
          fontSize: "1.25em",
          fontWeight: 600,
          textAlign: "center",
        }}
      >
        {now.getDate()}
      </div>

      <div
        style={{
          position: "absolute",
          right: -18,
          top: "56%",
          minWidth: 44,
          padding: "5px 9px",
          borderRadius: 999,
          background: "var(--layer3)",
          color: "var(--on-surface-variant)",
          fontVariantNumeric: "tabular-nums",
          fontSize: "0.95em",
          textAlign: "center",
        }}
        title={tr("Mo")}
      >
        {seconds.toString().padStart(2, "0")}
      </div>
    </div>
  );
}
