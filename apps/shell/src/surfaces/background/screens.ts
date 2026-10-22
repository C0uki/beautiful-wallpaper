// Where each monitor falls in the wallpaper surface.
//
// The surface is one window spread over every monitor, as Windows' own
// wallpaper is, and draws a wallpaper on each monitor's part of it. The
// monitors are known in physical pixels, from the top-left of the whole
// desktop; the page lays out in CSS pixels. The ratio between the two is taken
// from the window's own width rather than from `devicePixelRatio`: a window
// over monitors at different scales has one ratio, and only its size says
// which.

import type { MonitorInfo } from "@bw/core";

/** One monitor's part of the window, in CSS pixels. */
export interface Part {
  left: number;
  top: number;
  width: number;
  height: number;
  /** The widgets and the desktop's controls go on this one only. */
  primary: boolean;
}

export function monitorParts(
  monitors: MonitorInfo[],
  windowWidth: number,
  windowHeight: number,
): Part[] {
  // Nothing to go on, off Windows or before the answer: the whole window.
  if (!monitors.length) {
    return [
      {
        left: 0,
        top: 0,
        width: windowWidth,
        height: windowHeight,
        primary: true,
      },
    ];
  }
  const left = Math.min(...monitors.map((monitor) => monitor.x));
  const top = Math.min(...monitors.map((monitor) => monitor.y));
  const right = Math.max(
    ...monitors.map((monitor) => monitor.x + monitor.width),
  );
  const scale = windowWidth / Math.max(1, right - left);
  const named = monitors.some((monitor) => monitor.primary);
  return monitors.map((monitor, index) => ({
    left: (monitor.x - left) * scale,
    top: (monitor.y - top) * scale,
    width: monitor.width * scale,
    height: monitor.height * scale,
    primary: monitor.primary || (!named && index === 0),
  }));
}
