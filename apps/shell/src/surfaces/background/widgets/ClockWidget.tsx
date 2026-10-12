// The desktop clock.
//
// Two styles: a Mac's clock widget — a round face with the twelve hours, dark
// hands and an orange second hand — and a plain digital readout.

import { useEffect, useState } from "react";

/** The orange of a Mac's clock widget's second hand, in either appearance. */
const SECOND_HAND = "#ff9500";

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

export function ClockWidget({ style = "default" }: { style?: string }) {
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

  const size = 180;
  const radius = size / 2 - 8;
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
    <svg
      width={size}
      height={size}
      viewBox={`${-size / 2} ${-size / 2} ${size} ${size}`}
      style={{ filter: "drop-shadow(0 6px 18px var(--shadow))" }}
    >
      <circle
        r={radius}
        fill="var(--layer1)"
        stroke="var(--outline-variant)"
        strokeWidth={0.5}
      />
      {Array.from({ length: 12 }, (_, index) => {
        const angle = ((index + 1) / 12) * Math.PI * 2 - Math.PI / 2;
        return (
          <text
            key={index}
            x={Math.cos(angle) * radius * 0.78}
            y={Math.sin(angle) * radius * 0.78}
            textAnchor="middle"
            dominantBaseline="central"
            fontFamily="var(--font-title)"
            fontSize={radius * 0.2}
            fontWeight={500}
            fill="var(--on-surface)"
          >
            {index + 1}
          </text>
        );
      })}
      {hand(hours / 12, radius * 0.5, 6, "var(--on-surface)")}
      {hand(minutes / 60, radius * 0.72, 4, "var(--on-surface)")}
      {hand(seconds / 60, radius * 0.82, 1.5, SECOND_HAND)}
      <circle r={4} fill={SECOND_HAND} />
    </svg>
  );
}
