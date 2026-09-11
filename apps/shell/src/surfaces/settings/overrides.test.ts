// The settings form is generated from the Rust schema; OVERRIDES is the hand-
// written table that improves on it. Nothing but a test holds the two together,
// and every way they can drift apart fails quietly:
//
//   * a renamed config key leaves the override keyed to a path nobody asks for,
//     so the curated control silently reverts to the generated one;
//   * a `range` on a field the schema does not call a number is never reached,
//     because `Control` only consults it inside the integer/decimal case;
//   * a `choices` list that does not contain the value actually stored leaves a
//     `<select>` matching no `<option>` — the box shows the wrong setting, and
//     touching it writes a value in a spelling the backend does not use.

import { describe, expect, it } from "vitest";
import { configSchema, defaultConfig } from "@bw/core";
import { OVERRIDES } from "./overrides";

/** Reads a dotted path, the way `Control`'s caller does. */
function valueAt(config: unknown, path: string): unknown {
  return path
    .split(".")
    .reduce<unknown>(
      (node, key) =>
        node && typeof node === "object"
          ? (node as Record<string, unknown>)[key]
          : undefined,
      config,
    );
}

const kinds = new Map(configSchema.map((field) => [field.path, field.kind]));
const entries = Object.entries(OVERRIDES);

describe("the settings overrides", () => {
  it("only curates paths the config actually has", () => {
    const orphans = entries
      .map(([path]) => path)
      .filter((path) => !kinds.has(path));
    expect(
      orphans,
      "overrides keyed to a path no longer in the schema",
    ).toEqual([]);
  });

  it("puts a slider only where the schema has a number to slide", () => {
    const misplaced = entries
      .filter(([, override]) => override.range)
      .map(([path]) => path)
      .filter(
        (path) =>
          kinds.get(path) !== "integer" && kinds.get(path) !== "decimal",
      );
    expect(misplaced, "ranges on fields that are not numeric").toEqual([]);
  });

  it("gives every slider a range the default value sits inside", () => {
    for (const [path, override] of entries) {
      if (!override.range) continue;
      const { min, max, step } = override.range;
      expect(min, `${path} min/max`).toBeLessThan(max);
      expect(step, `${path} step`).toBeGreaterThan(0);

      // A default outside the track cannot be drawn: the handle pins to an end
      // and reports a setting the user never chose.
      const value = valueAt(defaultConfig, path);
      expect(typeof value, `${path} default`).toBe("number");
      expect(
        value as number,
        `${path} default is off the slider`,
      ).toBeGreaterThanOrEqual(min);
      expect(
        value as number,
        `${path} default is off the slider`,
      ).toBeLessThanOrEqual(max);
    }
  });

  it("offers the value the config actually ships with", () => {
    const unreachable = entries
      .filter(([path, override]) => {
        if (!override.choices) return false;
        const value = String(valueAt(defaultConfig, path));
        return !override.choices.some((choice) => choice.value === value);
      })
      .map(([path]) => `${path} (${String(valueAt(defaultConfig, path))})`);
    expect(
      unreachable,
      "defaults no option in the dropdown can represent",
    ).toEqual([]);
  });

  it("does not replace a control that is not a box to type in", () => {
    // `Control` returns the `<select>` before it ever looks at the kind, so
    // choices on a toggle would write a string where a bool belongs.
    const wrong = entries
      .filter(([, override]) => override.choices)
      .map(([path]) => path)
      .filter(
        (path) =>
          kinds.get(path) === "toggle" || kinds.get(path) === "textList",
      );
    expect(wrong, "choices on a field that is not text or a number").toEqual(
      [],
    );
  });

  it("labels every choice", () => {
    for (const [path, override] of entries) {
      for (const choice of override.choices ?? []) {
        expect(choice.label(), `${path} → ${choice.value}`).toBeTruthy();
      }
      if (override.hint) expect(override.hint(), `${path} hint`).toBeTruthy();
    }
  });
});
