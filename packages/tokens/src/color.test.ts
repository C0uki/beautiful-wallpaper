import { describe, expect, it } from "vitest";
import {
  applyAlpha,
  composite,
  contrastRatio,
  formatColor,
  hslLightness,
  hslSaturation,
  isDark,
  mix,
  parseColor,
  readableOn,
  solveOverlayColor,
  transparentize,
} from "./color";

describe("parsing and formatting", () => {
  it("reads every hex form", () => {
    expect(parseColor("#fff")).toEqual({ r: 1, g: 1, b: 1, a: 1 });
    expect(parseColor("#ffffff")).toEqual({ r: 1, g: 1, b: 1, a: 1 });
    expect(parseColor("#000000ff")).toEqual({ r: 0, g: 0, b: 0, a: 1 });
    expect(parseColor("#f00c").a).toBeCloseTo(0.8, 2);
  });

  it("reads rgb() and rgba()", () => {
    expect(parseColor("rgb(255, 0, 0)")).toEqual({ r: 1, g: 0, b: 0, a: 1 });
    expect(parseColor("rgba(0, 0, 255, 0.5)")).toEqual({
      r: 0,
      g: 0,
      b: 1,
      a: 0.5,
    });
  });

  it("rejects nonsense instead of silently returning black", () => {
    expect(() => parseColor("#12345")).toThrow();
    expect(() => parseColor("chartreuse")).toThrow();
    expect(() => parseColor("rgb(1, 2)")).toThrow();
  });

  it("round-trips through formatting", () => {
    for (const color of ["#7c4dff", "#000000", "#ffffff", "#123456"]) {
      expect(formatColor(parseColor(color))).toBe(color);
    }
  });

  it("only emits rgba() when the colour is translucent", () => {
    expect(formatColor({ r: 1, g: 0, b: 0, a: 1 })).toBe("#ff0000");
    expect(formatColor({ r: 1, g: 0, b: 0, a: 0.5 })).toBe(
      "rgba(255, 0, 0, 0.5)",
    );
  });
});

describe("mix", () => {
  it("returns the endpoints unchanged", () => {
    expect(mix("#000000", "#ffffff", 0)).toBe("#000000");
    expect(mix("#000000", "#ffffff", 1)).toBe("#ffffff");
  });

  it("interpolates linearly", () => {
    expect(mix("#000000", "#ffffff", 0.5)).toBe("#808080");
    expect(mix("#ff0000", "#0000ff", 0.25)).toBe("#bf0040");
  });

  it("clamps out-of-range amounts rather than extrapolating", () => {
    expect(mix("#000000", "#ffffff", 2)).toBe("#ffffff");
    expect(mix("#000000", "#ffffff", -1)).toBe("#000000");
  });
});

describe("alpha helpers", () => {
  it("transparentize scales the existing alpha", () => {
    expect(transparentize("#ff0000", 0.5)).toBe("rgba(255, 0, 0, 0.5)");
    // Applied twice, it compounds — it is a multiplier, not an assignment.
    expect(transparentize(transparentize("#ff0000", 0.5), 0.5)).toBe(
      "rgba(255, 0, 0, 0.25)",
    );
  });

  it("applyAlpha replaces it", () => {
    expect(applyAlpha("rgba(255, 0, 0, 0.2)", 0.8)).toBe(
      "rgba(255, 0, 0, 0.8)",
    );
  });
});

describe("luminance", () => {
  it("orders colours the way the eye does", () => {
    expect(isDark("#000000")).toBe(true);
    expect(isDark("#ffffff")).toBe(false);
    // Pure blue is dark despite being fully saturated; pure green is not.
    expect(isDark("#0000ff")).toBe(true);
    expect(isDark("#00ff00")).toBe(false);
  });

  it("computes WCAG contrast ratios", () => {
    expect(contrastRatio("#000000", "#ffffff")).toBeCloseTo(21, 5);
    expect(contrastRatio("#ffffff", "#ffffff")).toBeCloseTo(1, 5);
    // Order does not matter.
    expect(contrastRatio("#ffffff", "#000000")).toBeCloseTo(21, 5);
  });

  it("picks the more readable of two candidates", () => {
    expect(readableOn("#101010", "#ffffff", "#000000")).toBe("#ffffff");
    expect(readableOn("#f5f5f5", "#ffffff", "#000000")).toBe("#000000");
  });
});

describe("HSL accessors", () => {
  it("matches QML's hslLightness and hslSaturation", () => {
    expect(hslLightness("#000000")).toBe(0);
    expect(hslLightness("#ffffff")).toBe(1);
    expect(hslLightness("#ff0000")).toBeCloseTo(0.5, 5);
    expect(hslSaturation("#ff0000")).toBeCloseTo(1, 5);
    expect(hslSaturation("#808080")).toBe(0);
  });
});

describe("solveOverlayColor", () => {
  const cases: Array<[string, string, number]> = [
    ["#141218", "#211f26", 0.85],
    ["#000000", "#808080", 0.5],
    ["#ffffff", "#cccccc", 0.75],
    ["#2b1b2f", "#4a3350", 0.6],
  ];

  it("produces a paint that composites back to the target", () => {
    for (const [base, target, opacity] of cases) {
      const paint = solveOverlayColor(base, target, opacity);
      const result = parseColor(composite(base, paint));
      const wanted = parseColor(target);
      // Within one 8-bit step, which is all the round-trip can preserve.
      expect(Math.abs(result.r - wanted.r)).toBeLessThan(1 / 255);
      expect(Math.abs(result.g - wanted.g)).toBeLessThan(1 / 255);
      expect(Math.abs(result.b - wanted.b)).toBeLessThan(1 / 255);
    }
  });

  it("carries the requested opacity", () => {
    expect(
      parseColor(solveOverlayColor("#141218", "#211f26", 0.85)).a,
    ).toBeCloseTo(0.85, 5);
  });

  it("is the identity at full opacity", () => {
    expect(solveOverlayColor("#141218", "#211f26", 1)).toBe("#211f26");
  });

  it("clamps unreachable tones instead of wrapping", () => {
    // Asking for white over black at 10% opacity is impossible; the channel must
    // saturate at 1 rather than overflow into a wrong colour.
    const paint = parseColor(solveOverlayColor("#000000", "#ffffff", 0.1));
    expect(paint.r).toBe(1);
    expect(paint.g).toBe(1);
    expect(paint.b).toBe(1);
  });

  it("degrades gracefully at zero opacity", () => {
    expect(parseColor(solveOverlayColor("#000000", "#ff0000", 0)).a).toBe(0);
  });
});

describe("composite", () => {
  it("ignores a fully transparent overlay", () => {
    expect(composite("#123456", "rgba(255, 255, 255, 0)")).toBe("#123456");
  });

  it("replaces the base with a fully opaque overlay", () => {
    expect(composite("#123456", "#ff0000")).toBe("#ff0000");
  });

  it("blends at half opacity", () => {
    expect(composite("#000000", "rgba(255, 255, 255, 0.5)")).toBe("#808080");
  });
});
