// Motion and shape tokens.
//
// Easing is split into *spatial* curves (things that move) and *effects*
// curves (things that only fade or recolour). As on macOS, neither
// overshoots: a panel decelerates into place rather than bouncing past it.
// Corners are macOS's — small controls at 6, cards and popovers at 10, large
// panels at 16.

export const motion = {
  easing: {
    // Spatial — position, size, rotation.
    "spatial-fast": "cubic-bezier(0.25, 0.1, 0.25, 1.0)",
    "spatial-default": "cubic-bezier(0.2, 0.8, 0.2, 1.0)",
    "spatial-slow": "cubic-bezier(0.16, 1.0, 0.3, 1.0)",
    // Effects — opacity and colour, which never overshoot.
    "effects-fast": "cubic-bezier(0.31, 0.94, 0.34, 1.00)",
    "effects-default": "cubic-bezier(0.34, 0.80, 0.34, 1.00)",
    "effects-slow": "cubic-bezier(0.34, 0.88, 0.34, 1.00)",
    // The classic emphasized set, for large surfaces entering and leaving.
    emphasized: "cubic-bezier(0.2, 0.0, 0.0, 1.0)",
    "emphasized-decel": "cubic-bezier(0.05, 0.7, 0.1, 1.0)",
    "emphasized-accel": "cubic-bezier(0.3, 0.0, 0.8, 0.15)",
    standard: "cubic-bezier(0.2, 0.0, 0.0, 1.0)",
    linear: "linear",
  },
  duration: {
    quick: 100,
    fast: 150,
    normal: 200,
    slow: 300,
    slower: 400,
    /** Wallpaper crossfade, matching the original's 1200 ms. */
    wallpaper: 1200,
  },
} as const;

export const shape = {
  rounding: {
    unsharpen: 4,
    verysmall: 6,
    small: 8,
    normal: 10,
    large: 12,
    verylarge: 16,
    full: 9999,
  },
  /** Panel and widget sizing that several surfaces need to agree on. */
  sizes: {
    barHeight: 40,
    elevationMargin: 10,
    hoverPopupDelay: 300,
    widgetMinWidth: 180,
  },
} as const;

export type EasingName = keyof typeof motion.easing;
export type DurationName = keyof typeof motion.duration;
export type RoundingName = keyof typeof shape.rounding;

/** A ready-to-use `transition` value. */
export function transition(
  properties: string[],
  duration: DurationName = "normal",
  easing: EasingName = "effects-default",
): string {
  return properties
    .map(
      (property) =>
        `${property} var(--duration-${duration}) var(--ease-${easing})`,
    )
    .join(", ");
}
