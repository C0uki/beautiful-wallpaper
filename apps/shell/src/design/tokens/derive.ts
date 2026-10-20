// The derived token layer: end4-pC's `Appearance.qml`.
//
// The generated scheme gives raw Material roles. Almost nothing in the UI binds
// to those directly — it binds to *layers* (`layer0`..`layer4`, each with hover,
// active, disabled and border variants) so a button looks right whether it sits
// on the bar, in a popup or on a card. That derivation is what this file is.

import type { GeneratedTheme } from "@bw/core";
import { applyAlpha, mix, solveOverlayColor, transparentize } from "./color";

/** State variants every layer carries. */
export interface LayerColors {
  base: string;
  hover: string;
  active: string;
  disabled: string;
  border: string;
  on: string;
}

export interface DerivedTokens {
  mode: "light" | "dark";
  /** Raw Material roles, camelCased from the generated snake_case keys. */
  m3: Record<string, string>;
  /** Elevation-ish layers, from the page background up to the highest surface. */
  layer: [LayerColors, LayerColors, LayerColors, LayerColors, LayerColors];
  primary: LayerColors;
  secondary: LayerColors;
  tertiary: LayerColors;
  error: LayerColors;
  success: LayerColors;
  /** Text and icon colours. */
  onSurface: string;
  onSurfaceVariant: string;
  outline: string;
  outlineVariant: string;
  scrim: string;
  shadow: string;
  /** How translucent panel backgrounds should be, in 0..1. */
  backgroundTransparency: number;
  /** 16 terminal colours, for the Windows Terminal scheme export. */
  terminal: string[];
}

export interface DeriveOptions {
  /** From the config's `appearance.transparency`. */
  transparencyEnabled?: boolean;
  extraTransparency?: number;
}

/**
 * Wallpaper vibrancy → background translucency.
 *
 * end4-pC fits `y = 0.5768x² − 0.759x + 0.2896` to hand-tuned values and clamps
 * to `[0, 0.22]`: washed-out wallpapers get a more opaque panel so text stays
 * legible, vivid ones get a more transparent one so the wallpaper shows through.
 */
export function autoBackgroundTransparency(vibrancy: number): number {
  const x = Math.min(1, Math.max(0, vibrancy));
  const y = 0.5768 * x * x - 0.759 * x + 0.2896;
  return Math.min(0.22, Math.max(0, y));
}

/** `surface_container_high` → `surfaceContainerHigh`. */
export function camelCase(key: string): string {
  return key.replace(/_([a-z0-9])/g, (_, char: string) => char.toUpperCase());
}

function role(
  m3: Record<string, string>,
  name: string,
  fallback: string,
): string {
  return m3[name] ?? fallback;
}

/** Builds one layer's state variants from a base and the text colour on it. */
function layerFrom(
  base: string,
  on: string,
  mode: "light" | "dark",
): LayerColors {
  // Material state layers are the "on" colour at 8% / 12% opacity over the base.
  // Compositing here rather than stacking a translucent element keeps the result
  // correct when the layer underneath is itself translucent.
  const hover = mix(base, on, mode === "dark" ? 0.08 : 0.06);
  const active = mix(base, on, mode === "dark" ? 0.12 : 0.1);
  return {
    base,
    hover,
    active,
    disabled: transparentize(base, 0.62),
    border: applyAlpha(on, 0.12),
    on,
  };
}

export function deriveTokens(
  theme: GeneratedTheme,
  options: DeriveOptions = {},
): DerivedTokens {
  const m3: Record<string, string> = {};
  for (const [key, value] of Object.entries(theme.colors)) {
    m3[camelCase(key)] = value;
  }

  const mode = theme.mode;
  const onSurface = role(
    m3,
    "onSurface",
    mode === "dark" ? "#e6e0e9" : "#1d1b20",
  );
  const onSurfaceVariant = role(m3, "onSurfaceVariant", onSurface);

  const transparencyEnabled = options.transparencyEnabled ?? true;
  const backgroundTransparency = transparencyEnabled
    ? Math.min(
        0.9,
        Math.max(
          0,
          autoBackgroundTransparency(theme.wallpaperVibrancy) +
            (options.extraTransparency ?? 0),
        ),
      )
    : 0;

  // Layer 0 is the page itself and the one that shows the wallpaper through, so
  // it is the only layer solved against a translucent paint.
  const surface = role(m3, "surface", "#141218");
  const layer0Base =
    backgroundTransparency > 0
      ? solveOverlayColor(surface, surface, 1 - backgroundTransparency)
      : surface;

  const layers: [string, string, string, string, string] = [
    layer0Base,
    role(m3, "surfaceContainerLow", surface),
    role(m3, "surfaceContainer", surface),
    role(m3, "surfaceContainerHigh", surface),
    role(m3, "surfaceContainerHighest", surface),
  ];

  const pair = (color: string, on: string) => layerFrom(color, on, mode);

  return {
    mode,
    m3,
    layer: layers.map((base) =>
      layerFrom(base, onSurface, mode),
    ) as DerivedTokens["layer"],
    primary: pair(
      role(m3, "primary", "#d0bcff"),
      role(m3, "onPrimary", "#381e72"),
    ),
    secondary: pair(
      role(m3, "secondary", "#ccc2dc"),
      role(m3, "onSecondary", "#332d41"),
    ),
    tertiary: pair(
      role(m3, "tertiary", "#efb8c8"),
      role(m3, "onTertiary", "#492532"),
    ),
    error: pair(role(m3, "error", "#f2b8b5"), role(m3, "onError", "#601410")),
    success: pair(
      role(m3, "success", "#7ad67a"),
      role(m3, "onSuccess", "#00390a"),
    ),
    onSurface,
    onSurfaceVariant,
    outline: role(m3, "outline", onSurfaceVariant),
    outlineVariant: role(m3, "outlineVariant", onSurfaceVariant),
    scrim: applyAlpha(role(m3, "scrim", "#000000"), 0.55),
    shadow: applyAlpha(role(m3, "shadow", "#000000"), 0.4),
    backgroundTransparency,
    terminal: Array.from(
      { length: 16 },
      (_, index) => m3[`term${index}`] ?? onSurface,
    ),
  };
}
