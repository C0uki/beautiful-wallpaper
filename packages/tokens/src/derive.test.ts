import type { GeneratedTheme } from "@bw/core";
import { describe, expect, it } from "vitest";
import { autoBackgroundTransparency, camelCase, deriveTokens } from "./derive";
import { cssVariables, cssVariablesText } from "./css";

function theme(overrides: Partial<GeneratedTheme> = {}): GeneratedTheme {
  return {
    mode: "dark",
    variant: "tonalSpot",
    source: "#7c4dff",
    wallpaperVibrancy: 0.5,
    colors: {
      primary: "#d0bcff",
      on_primary: "#381e72",
      secondary: "#ccc2dc",
      on_secondary: "#332d41",
      tertiary: "#efb8c8",
      on_tertiary: "#492532",
      error: "#f2b8b5",
      on_error: "#601410",
      success: "#7ad67a",
      on_success: "#00390a",
      surface: "#141218",
      on_surface: "#e6e0e9",
      on_surface_variant: "#cac4d0",
      surface_container_low: "#1d1b20",
      surface_container: "#211f26",
      surface_container_high: "#2b2930",
      surface_container_highest: "#36343b",
      outline: "#938f99",
      outline_variant: "#49454f",
      scrim: "#000000",
      shadow: "#000000",
      term0: "#282828",
      term15: "#ebdbb2",
    },
    ...overrides,
  };
}

describe("camelCase", () => {
  it("matches the QML key rewrite", () => {
    expect(camelCase("surface_container_high")).toBe("surfaceContainerHigh");
    expect(camelCase("on_primary_fixed_variant")).toBe("onPrimaryFixedVariant");
    expect(camelCase("primary")).toBe("primary");
    expect(camelCase("term0")).toBe("term0");
  });
});

describe("autoBackgroundTransparency", () => {
  it("follows the fitted curve from Appearance.qml", () => {
    // y = 0.5768x² − 0.759x + 0.2896, clamped to [0, 0.22].
    // At x = 0 the curve gives 0.2896, above the cap, so it clamps.
    expect(autoBackgroundTransparency(0)).toBe(0.22);
    expect(autoBackgroundTransparency(0.1)).toBeCloseTo(0.2195, 4);
    expect(autoBackgroundTransparency(0.5)).toBeCloseTo(0.0543, 4);
    expect(autoBackgroundTransparency(1)).toBeCloseTo(0.1074, 4);
  });

  it("clamps to the documented range", () => {
    for (let x = -1; x <= 2; x += 0.05) {
      const value = autoBackgroundTransparency(x);
      expect(value).toBeGreaterThanOrEqual(0);
      expect(value).toBeLessThanOrEqual(0.22);
    }
  });

  it("makes vivid wallpapers less opaque than washed-out ones", () => {
    expect(autoBackgroundTransparency(0.55)).toBeLessThan(
      autoBackgroundTransparency(0.05),
    );
  });
});

describe("deriveTokens", () => {
  it("camelCases every Material role", () => {
    const tokens = deriveTokens(theme());
    expect(tokens.m3["surfaceContainerHigh"]).toBe("#2b2930");
    expect(tokens.m3["onSurfaceVariant"]).toBe("#cac4d0");
    expect(tokens.m3["surface_container_high"]).toBeUndefined();
  });

  it("builds five layers, each with its state variants", () => {
    const tokens = deriveTokens(theme());
    expect(tokens.layer).toHaveLength(5);
    for (const layer of tokens.layer) {
      for (const key of [
        "base",
        "hover",
        "active",
        "disabled",
        "border",
        "on",
      ] as const) {
        expect(layer[key]).toMatch(/^(#|rgba\()/);
      }
      // Hover must be a visible step away from the base, and active further still.
      expect(layer.hover).not.toBe(layer.base);
      expect(layer.active).not.toBe(layer.hover);
    }
  });

  it("orders the layers from the page background upwards", () => {
    const tokens = deriveTokens(theme(), { transparencyEnabled: false });
    const bases = tokens.layer.map((layer) => layer.base);
    expect(bases[0]).toBe("#141218");
    expect(bases[4]).toBe("#36343b");
  });

  it("uses the wallpaper vibrancy for background transparency", () => {
    const vivid = deriveTokens(theme({ wallpaperVibrancy: 0.55 }));
    const washed = deriveTokens(theme({ wallpaperVibrancy: 0.02 }));
    expect(vivid.backgroundTransparency).toBeLessThan(
      washed.backgroundTransparency,
    );
  });

  it("honours the transparency switch and the extra offset", () => {
    expect(
      deriveTokens(theme(), { transparencyEnabled: false })
        .backgroundTransparency,
    ).toBe(0);
    const boosted = deriveTokens(theme(), { extraTransparency: 0.3 });
    const plain = deriveTokens(theme());
    expect(boosted.backgroundTransparency).toBeCloseTo(
      plain.backgroundTransparency + 0.3,
      5,
    );
  });

  it("falls back for roles a scheme leaves out", () => {
    const sparse = deriveTokens(theme({ colors: { primary: "#ff0000" } }));
    expect(sparse.primary.base).toBe("#ff0000");
    // Missing roles must still produce a usable colour rather than `undefined`.
    expect(sparse.onSurface).toMatch(/^#/);
    expect(sparse.layer[3]!.base).toMatch(/^(#|rgba\()/);
    expect(sparse.terminal).toHaveLength(16);
  });

  it("exposes all sixteen terminal colours", () => {
    const tokens = deriveTokens(theme());
    expect(tokens.terminal[0]).toBe("#282828");
    expect(tokens.terminal[15]).toBe("#ebdbb2");
  });
});

describe("cssVariables", () => {
  it("kebab-cases Material roles under an --m3- prefix", () => {
    const variables = cssVariables(deriveTokens(theme()));
    expect(variables["--m3-surface-container-high"]).toBe("#2b2930");
    expect(variables["--m3-on-surface-variant"]).toBe("#cac4d0");
  });

  it("emits every layer and semantic slot", () => {
    const variables = cssVariables(deriveTokens(theme()));
    for (const name of [
      "--layer0",
      "--layer0-hover",
      "--layer4-border",
      "--primary",
      "--primary-on",
      "--error-active",
      "--success",
      "--on-surface",
      "--outline",
      "--scrim",
      "--term-15",
    ]) {
      expect(variables[name], name).toBeDefined();
    }
  });

  it("includes motion and shape tokens so CSS never hardcodes them", () => {
    const variables = cssVariables(deriveTokens(theme()));
    expect(variables["--ease-spatial-default"]).toContain("cubic-bezier");
    expect(variables["--duration-wallpaper"]).toBe("1200ms");
    expect(variables["--rounding-normal"]).toBe("16px");
  });

  it("renders as a stylesheet", () => {
    const text = cssVariablesText(deriveTokens(theme()));
    expect(text.startsWith(":root {")).toBe(true);
    expect(text).toContain("--m3-primary: #d0bcff;");
    expect(text.trimEnd().endsWith("}")).toBe(true);
  });
});
