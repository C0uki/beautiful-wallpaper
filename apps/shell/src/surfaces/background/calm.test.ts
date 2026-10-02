import { describe, expect, it } from "vitest";
import { busyness, calmestSpot } from "./calm";

/** A 40 × 30 picture of noise with one flat 10 × 8 patch at (24, 4). */
function picture() {
  const width = 40;
  const height = 30;
  const luma = new Float32Array(width * height);
  for (let i = 0; i < luma.length; i++) luma[i] = (i * 7919) % 13;
  for (let y = 4; y < 12; y++) {
    for (let x = 24; x < 34; x++) luma[y * width + x] = 5;
  }
  return { busy: busyness(luma, width, height), width, height };
}

describe("finding the calmest part of a wallpaper", () => {
  it("puts a box that fits inside the flat patch inside it", () => {
    const { busy, width, height } = picture();
    const spot = calmestSpot(busy, width, height, 6, 4, []);
    expect(spot.x).toBeGreaterThanOrEqual(24);
    expect(spot.x + 6).toBeLessThanOrEqual(34);
    expect(spot.y).toBeGreaterThanOrEqual(4);
    expect(spot.y + 4).toBeLessThanOrEqual(12);
  });

  it("goes somewhere else when the patch is taken, and keeps its margin", () => {
    const { busy, width, height } = picture();
    const taken = [{ x: 22, y: 2, width: 14, height: 12 }];
    const spot = calmestSpot(busy, width, height, 6, 4, taken, 2);
    expect(
      spot.x + 6 <= 22 || spot.x >= 36 || spot.y + 4 <= 2 || spot.y >= 14,
    ).toBe(true);
    expect(spot.x).toBeGreaterThanOrEqual(2);
    expect(spot.y + 4).toBeLessThanOrEqual(height - 2);
  });

  it("still answers for a box larger than the picture", () => {
    const { busy, width, height } = picture();
    expect(calmestSpot(busy, width, height, 50, 40, [])).toEqual({
      x: 0,
      y: 0,
    });
  });
});
