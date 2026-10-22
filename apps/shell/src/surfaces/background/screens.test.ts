import { describe, expect, it } from "vitest";
import type { MonitorInfo } from "@bw/core";
import { monitorParts } from "./screens";

const monitor = (
  x: number,
  y: number,
  width: number,
  height: number,
  primary = false,
): MonitorInfo => ({
  name: "",
  x,
  y,
  width,
  height,
  workWidth: width,
  workHeight: height,
  primary,
});

describe("where each monitor falls in the wallpaper surface", () => {
  it("lays a second monitor to the left of the primary one out first", () => {
    // 2560 + 1920 physical pixels across, drawn 2988 CSS pixels wide (1.5).
    const parts = monitorParts(
      [monitor(0, 0, 1920, 1080, true), monitor(-2560, -360, 2560, 1440)],
      2986.6666,
      1200,
    );
    const [primary, left] = parts;
    expect(primary!.primary).toBe(true);
    expect(left!.primary).toBe(false);
    expect(left!.left).toBe(0);
    expect(left!.top).toBe(0);
    expect(primary!.left).toBeCloseTo(2560 / 1.5, 1);
    expect(primary!.top).toBeCloseTo(360 / 1.5, 1);
    expect(primary!.width).toBeCloseTo(1920 / 1.5, 1);
  });

  it("covers the whole window when there is nothing to go on", () => {
    expect(monitorParts([], 800, 600)).toEqual([
      { left: 0, top: 0, width: 800, height: 600, primary: true },
    ]);
  });

  it("puts the widgets somewhere even when no monitor is called primary", () => {
    const parts = monitorParts(
      [monitor(0, 0, 100, 100), monitor(100, 0, 100, 100)],
      200,
      100,
    );
    expect(parts.map((part) => part.primary)).toEqual([true, false]);
  });
});
