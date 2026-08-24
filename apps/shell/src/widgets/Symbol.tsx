// A Material Symbols glyph.
//
// The font selects icons by ligature, so the icon name is the element's text.
// `filled` animates the FILL axis, which is the standard Material way of showing
// a toggle turning on.

import type { CSSProperties } from "react";

export interface SymbolProps {
  name: string;
  /** Font size in pixels. */
  size?: number;
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
  return (
    <span
      className={className ? `symbol ${className}` : "symbol"}
      aria-hidden="true"
      style={{
        fontSize: size,
        width: size,
        height: size,
        display: "inline-flex",
        alignItems: "center",
        justifyContent: "center",
        color,
        transition:
          "font-variation-settings var(--duration-normal) var(--ease-effects-default)",
        ...({
          "--symbol-fill": filled ? 1 : 0,
          "--symbol-weight": weight,
        } as CSSProperties),
        ...style,
      }}
    >
      {name}
    </span>
  );
}
