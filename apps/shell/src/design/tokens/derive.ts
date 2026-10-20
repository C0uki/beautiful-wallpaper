// The derived token layer.
//
// The UI binds to *layers* (`layer0`..`layer4`, each with hover, active,
// disabled and border variants) so a button looks right whether it sits on the
// bar, in a popup or on a card. Those layers are macOS's: neutral greys for
// the window, the controls on it and the fills inside them, label and
// separator greys for text and lines, and one accent — Apple's blue, a colour
// of the user's choosing, or the wallpaper's. The generated Material scheme
// is still passed through under `m3`, and is where the wallpaper's accent
// comes from.

import type { GeneratedTheme } from "@bw/core";
import {
  applyAlpha,
  contrastRatio,
  mix,
  solveOverlayColor,
  transparentize,
} from "./color";

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
  /** `appearance.palette.accentColor`: the accent itself. Unset is Apple's
   *  blue; empty takes the wallpaper's, as the scheme's primary. */
  accent?: string | null;
}

/** macOS's neutral greys and system colours, per appearance. */
const APPLE = {
  dark: {
    // windowBackground, controlBackground, then the fills inside a control.
    layers: ["#1e1e1e", "#282828", "#323232", "#3c3c3c", "#464646"],
    label: "rgba(255, 255, 255, 0.85)",
    secondaryLabel: "rgba(255, 255, 255, 0.55)",
    separator: "rgba(255, 255, 255, 0.1)",
    grid: "rgba(255, 255, 255, 0.24)",
    blue: "#0a84ff",
    red: "#ff453a",
    green: "#30d158",
    purple: "#bf5af2",
  },
  light: {
    layers: ["#ececec", "#ffffff", "#f2f2f2", "#e6e6e6", "#dadada"],
    label: "rgba(0, 0, 0, 0.85)",
    secondaryLabel: "rgba(0, 0, 0, 0.5)",
    separator: "rgba(0, 0, 0, 0.1)",
    grid: "rgba(0, 0, 0, 0.22)",
    blue: "#007aff",
    red: "#ff3b30",
    green: "#34c759",
    purple: "#af52de",
  },
} as const;

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
  // A grey gets one fill step lighter (dark) or darker (light) under the
  // pointer, and another when pressed. Compositing here rather than stacking a
  // translucent element keeps the result correct when the layer underneath is
  // itself translucent.
  const toward = mode === "dark" ? "#ffffff" : "#000000";
  const hover = mix(base, toward, 0.06);
  const active = mix(base, toward, 0.12);
  return {
    base,
    hover,
    active,
    disabled: transparentize(base, 0.62),
    border: applyAlpha(on, 0.12),
    on,
  };
}

/** A coloured control: as macOS does, it darkens under the pointer and
 *  further when pressed, and its text is white unless that cannot be read.
 *  Not whichever contrasts more: black edges out white on Apple's blue, and
 *  a Mac still writes white on it. 3:1 is the large-text and UI floor. */
function tint(base: string): LayerColors {
  const on = contrastRatio(base, "#ffffff") >= 3 ? "#ffffff" : "#000000";
  return {
    base,
    hover: mix(base, "#000000", 0.08),
    active: mix(base, "#000000", 0.16),
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
  const apple = APPLE[mode];
  const onSurface = apple.label;
  const onSurfaceVariant = apple.secondaryLabel;

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

  // Layer 0 is the window itself and the one that shows the wallpaper
  // through, so it is the only layer solved against a translucent paint.
  const [page, ...rest] = apple.layers;
  const layer0Base =
    backgroundTransparency > 0
      ? solveOverlayColor(page, page, 1 - backgroundTransparency)
      : page;
  const layers = [layer0Base, ...rest];

  // Apple's blue until another is chosen; empty is the wallpaper's, as the
  // scheme's primary, which Material has already made legible in this
  // appearance.
  const chosen = options.accent?.trim();
  const accent =
    chosen === undefined
      ? apple.blue
      : chosen || role(m3, "primary", apple.blue);

  return {
    mode,
    m3,
    layer: layers.map((base) =>
      layerFrom(base, onSurface, mode),
    ) as DerivedTokens["layer"],
    primary: tint(accent),
    // A quieter accent: the accent washed into the control background, for
    // the selected row or the icon's circle that should not shout.
    secondary: layerFrom(mix(apple.layers[1], accent, 0.3), onSurface, mode),
    tertiary: tint(apple.purple),
    error: tint(apple.red),
    success: tint(apple.green),
    onSurface,
    onSurfaceVariant,
    outline: apple.grid,
    outlineVariant: apple.separator,
    scrim: applyAlpha("#000000", mode === "dark" ? 0.5 : 0.3),
    shadow: applyAlpha("#000000", mode === "dark" ? 0.5 : 0.2),
    backgroundTransparency,
    terminal: Array.from(
      { length: 16 },
      (_, index) => m3[`term${index}`] ?? onSurface,
    ),
  };
}
