// Formatting shared across surfaces.
//
// These lived in the bar and in the desktop widgets separately; the sidebar and
// the toasts would have made a third and fourth copy.

const RATE_UNITS = ["B", "K", "M", "G"];
const SIZE_UNITS = ["B", "KB", "MB", "GB", "TB"];

function scale(value: number, units: string[]): [number, string] {
  let scaled = Math.max(value, 0);
  let unit = 0;
  while (scaled >= 1024 && unit < units.length - 1) {
    scaled /= 1024;
    unit += 1;
  }
  return [scaled, units[unit]!];
}

/** Bytes per second, in the shortest form that stays readable. */
export function formatRate(bytesPerSecond: number): string {
  const [value, unit] = scale(bytesPerSecond, RATE_UNITS);
  const rounded =
    value < 10 && unit !== "B" ? value.toFixed(1) : String(Math.round(value));
  return `${rounded}${unit}`;
}

export function formatBytes(bytes: number): string {
  const [value, unit] = scale(bytes, SIZE_UNITS);
  return `${value.toFixed(value >= 100 || unit === "B" ? 0 : 1)} ${unit}`;
}

/** "now", "3m", "2h" — how long ago something happened, at a glance. */
export function formatAge(
  seconds: number,
  now: number = Date.now() / 1000,
): string {
  const elapsed = Math.max(0, Math.floor(now - seconds));
  if (elapsed < 45) return "now";
  if (elapsed < 3600) return `${Math.round(elapsed / 60)}m`;
  if (elapsed < 86_400) return `${Math.round(elapsed / 3600)}h`;
  return `${Math.round(elapsed / 86_400)}d`;
}
