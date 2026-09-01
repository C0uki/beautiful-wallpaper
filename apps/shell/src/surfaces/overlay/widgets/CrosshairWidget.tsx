// The crosshair.
//
// Four inner arms, four outer arms and a dot, all built from the share code
// the game produces. The code is read in Rust — the format is a patch on the
// game's defaults with three separate traps in it — so this file only draws
// what it is handed.
//
// Every arm is one box rotated into place rather than four hand-written
// positions: the rotations are what guarantee the four are the same length as
// each other, which is the thing a crosshair is most obviously wrong about.

import type { Crosshair } from "@bw/core";

export function CrosshairWidget({ crosshair }: { crosshair: Crosshair }) {
  const border = crosshair.outline ? crosshair.outlineThickness : 0;
  const outlineColor = `rgba(0, 0, 0, ${crosshair.outlineOpacity})`;

  const arm = (kind: "inner" | "outer", index: number): React.CSSProperties => {
    const horizontal = index % 2 === 0;
    const length = horizontal
      ? kind === "inner"
        ? crosshair.innerLineLength
        : crosshair.outerLineLength
      : kind === "inner"
        ? crosshair.innerLineVerticalLength
        : crosshair.outerLineVerticalLength;
    const thickness =
      kind === "inner"
        ? crosshair.innerLineThickness
        : crosshair.outerLineThickness;
    const offset =
      kind === "inner" ? crosshair.innerLineOffset : crosshair.outerLineOffset;

    return {
      position: "absolute",
      left: "50%",
      top: "50%",
      width: length + border * 2,
      height: thickness + border * 2,
      // Rotated about the centre, then pushed out along its own axis, so all
      // four arms are the same shape by construction.
      transform: `rotate(${index * 90}deg) translate(${offset - border}px, -50%)`,
      transformOrigin: "0 0",
      background: crosshair.color,
      opacity:
        kind === "inner"
          ? crosshair.innerLineOpacity
          : crosshair.outerLineOpacity,
      boxShadow: border ? `0 0 0 ${border}px ${outlineColor}` : undefined,
      display: length > 0 ? "block" : "none",
    };
  };

  return (
    <div
      className="bw-crosshair"
      style={{ width: crosshair.size, height: crosshair.size }}
    >
      {crosshair.centerDot ? (
        <span
          className="bw-crosshair-dot"
          style={{
            width: crosshair.centerDotSize + border * 2,
            height: crosshair.centerDotSize + border * 2,
            background: crosshair.color,
            opacity: crosshair.centerDotOpacity,
            boxShadow: border ? `0 0 0 ${border}px ${outlineColor}` : undefined,
          }}
        />
      ) : null}

      {crosshair.innerLines
        ? [0, 1, 2, 3].map((index) => (
            <span key={`inner-${index}`} style={arm("inner", index)} />
          ))
        : null}
      {crosshair.outerLines
        ? [0, 1, 2, 3].map((index) => (
            <span key={`outer-${index}`} style={arm("outer", index)} />
          ))
        : null}
    </div>
  );
}
