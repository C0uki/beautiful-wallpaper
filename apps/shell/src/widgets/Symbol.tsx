// An icon, drawn as an inline SVG from the Lucide set (see scripts/icons.json).
//
// Names are the ones the code has always used — Material Symbols names, which
// the Rust side sends too — and icons.json says which Lucide icon each draws.
// The stroke is kept near a 1.5px hairline at any size, as SF Symbols are in
// a Mac's menus, rather than scaling with the icon. `filled` thickens the
// stroke: Lucide has no filled variants, and filling an open path turns arcs
// like the Wi-Fi bars into wedges. A control shows "on" with its own
// background, as macOS does.

import type { CSSProperties } from "react";
import { ICONS } from "./icons.gen";

export interface SymbolProps {
  name: string;
  /** Size in pixels. */
  size?: number;
  /** 400 is the normal stroke; lower is lighter, higher heavier. */
  weight?: number;
  filled?: boolean;
  color?: string;
  className?: string;
  style?: CSSProperties;
}

export function Symbol({
  name,
  size = 22,
  weight = 400,
  filled = false,
  color,
  className,
  style,
}: SymbolProps) {
  // In the 24-unit viewBox: 1.5px on screen, scaled by the weight.
  const stroke = ((1.5 * 24) / size) * (weight / 400) * (filled ? 1.35 : 1);
  return (
    <svg
      className={className ? `symbol ${className}` : "symbol"}
      aria-hidden="true"
      viewBox="0 0 24 24"
      width={size}
      height={size}
      fill="none"
      stroke="currentColor"
      strokeWidth={stroke}
      strokeLinecap="round"
      strokeLinejoin="round"
      style={{
        flex: "none",
        color,
        transition:
          "stroke-width var(--duration-normal) var(--ease-effects-default)",
        ...style,
      }}
      // An unknown name draws nothing rather than a word; icons.test.ts is
      // what keeps every name the code uses in the map.
      dangerouslySetInnerHTML={{ __html: ICONS[name] ?? "" }}
    />
  );
}
