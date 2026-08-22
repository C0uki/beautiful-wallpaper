// Colour maths, ported from end4-pC's `modules/common/functions/ColorUtils.qml`.
//
// The interesting one is `solveOverlayColor`: Material tones are specified as
// the colour you should *see*, but the shell paints translucent layers over a
// wallpaper. Given the intended tone and the layer's opacity, it algebraically
// inverts alpha compositing to find the colour to actually paint.

export interface Rgba {
  r: number;
  g: number;
  b: number;
  a: number;
}

const clamp = (value: number, low = 0, high = 1) =>
  Math.min(high, Math.max(low, value));

const hex2 = (value: number) =>
  Math.round(clamp(value) * 255)
    .toString(16)
    .padStart(2, "0");

/** Parses `#rgb`, `#rrggbb`, `#rrggbbaa` or `rgba(...)`. */
export function parseColor(input: string): Rgba {
  const text = input.trim();

  if (text.startsWith("#")) {
    const digits = text.slice(1);
    const expand = (part: string) =>
      parseInt(part.length === 1 ? part + part : part, 16) / 255;
    if (digits.length === 3 || digits.length === 4) {
      return {
        r: expand(digits[0]!),
        g: expand(digits[1]!),
        b: expand(digits[2]!),
        a: digits.length === 4 ? expand(digits[3]!) : 1,
      };
    }
    if (digits.length === 6 || digits.length === 8) {
      const pair = (at: number) => parseInt(digits.slice(at, at + 2), 16) / 255;
      return {
        r: pair(0),
        g: pair(2),
        b: pair(4),
        a: digits.length === 8 ? pair(6) : 1,
      };
    }
    throw new Error(`not a hex colour: ${input}`);
  }

  const functional = /^rgba?\(([^)]+)\)$/i.exec(text);
  if (functional) {
    const parts = functional[1]!.split(/[,/\s]+/).filter(Boolean);
    if (parts.length < 3) throw new Error(`not a colour: ${input}`);
    const channel = (part: string) =>
      part.endsWith("%") ? parseFloat(part) / 100 : parseFloat(part) / 255;
    return {
      r: channel(parts[0]!),
      g: channel(parts[1]!),
      b: channel(parts[2]!),
      a: parts[3] === undefined ? 1 : parseFloat(parts[3]),
    };
  }

  throw new Error(`not a colour: ${input}`);
}

/** Serialises back to `#rrggbb`, or `rgba(...)` when translucent. */
export function formatColor(color: Rgba): string {
  if (color.a >= 1) {
    return `#${hex2(color.r)}${hex2(color.g)}${hex2(color.b)}`;
  }
  const channel = (value: number) => Math.round(clamp(value) * 255);
  const alpha = Math.round(clamp(color.a) * 1000) / 1000;
  return `rgba(${channel(color.r)}, ${channel(color.g)}, ${channel(color.b)}, ${alpha})`;
}

/** Linear interpolation between two colours; `amount` 0 keeps `a`. */
export function mix(a: string, b: string, amount: number): string {
  const from = parseColor(a);
  const to = parseColor(b);
  const t = clamp(amount);
  return formatColor({
    r: from.r + (to.r - from.r) * t,
    g: from.g + (to.g - from.g) * t,
    b: from.b + (to.b - from.b) * t,
    a: from.a + (to.a - from.a) * t,
  });
}

/** Multiplies the existing alpha — `transparentize(c, 0.5)` halves it. */
export function transparentize(color: string, amount: number): string {
  const parsed = parseColor(color);
  return formatColor({ ...parsed, a: clamp(parsed.a * (1 - clamp(amount))) });
}

/** Replaces the alpha outright. */
export function applyAlpha(color: string, alpha: number): string {
  return formatColor({ ...parseColor(color), a: clamp(alpha) });
}

/** Relative luminance, per WCAG. */
export function luminance(color: string): number {
  const { r, g, b } = parseColor(color);
  const linear = (channel: number) =>
    channel <= 0.03928 ? channel / 12.92 : ((channel + 0.055) / 1.055) ** 2.4;
  return 0.2126 * linear(r) + 0.7152 * linear(g) + 0.0722 * linear(b);
}

export function isDark(color: string): boolean {
  return luminance(color) < 0.5;
}

/** Contrast ratio between two opaque colours, 1..21. */
export function contrastRatio(a: string, b: string): number {
  const [high, low] = [luminance(a), luminance(b)].sort((x, y) => y - x) as [
    number,
    number,
  ];
  return (high + 0.05) / (low + 0.05);
}

/** Picks whichever of `light`/`dark` reads better on `background`. */
export function readableOn(
  background: string,
  light: string,
  dark: string,
): string {
  return contrastRatio(background, light) >= contrastRatio(background, dark)
    ? light
    : dark;
}

/** HSL lightness, matching QML's `color.hslLightness`. */
export function hslLightness(color: string): number {
  const { r, g, b } = parseColor(color);
  return (Math.max(r, g, b) + Math.min(r, g, b)) / 2;
}

/** HSL saturation, matching QML's `color.hslSaturation`. */
export function hslSaturation(color: string): number {
  const { r, g, b } = parseColor(color);
  const max = Math.max(r, g, b);
  const min = Math.min(r, g, b);
  if (max === min) return 0;
  const lightness = (max + min) / 2;
  return (max - min) / (1 - Math.abs(2 * lightness - 1));
}

/**
 * The colour to paint at `opacity` over `base` so the result looks like `target`.
 *
 * Alpha compositing gives `result = paint·α + base·(1−α)`; solving for `paint`
 * gives `(target − base·(1−α)) / α`. When the required channel falls outside
 * 0..1 the tone is unreachable at that opacity, and the closest reachable colour
 * is used instead of letting the value wrap.
 */
export function solveOverlayColor(
  base: string,
  target: string,
  opacity: number,
): string {
  const alpha = clamp(opacity);
  if (alpha <= 0) {
    // Nothing painted at zero opacity can change the result; return the target
    // so callers still get a sensible colour to fall back on.
    return formatColor({ ...parseColor(target), a: 0 });
  }

  const under = parseColor(base);
  const want = parseColor(target);
  const solve = (wanted: number, beneath: number) =>
    clamp((wanted - beneath * (1 - alpha)) / alpha);

  return formatColor({
    r: solve(want.r, under.r),
    g: solve(want.g, under.g),
    b: solve(want.b, under.b),
    a: alpha,
  });
}

/** Composites `over` onto `under`, the operation `solveOverlayColor` inverts. */
export function composite(under: string, over: string): string {
  const below = parseColor(under);
  const above = parseColor(over);
  const alpha = clamp(above.a);
  return formatColor({
    r: above.r * alpha + below.r * (1 - alpha),
    g: above.g * alpha + below.g * (1 - alpha),
    b: above.b * alpha + below.b * (1 - alpha),
    a: 1,
  });
}
