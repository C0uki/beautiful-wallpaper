import { describe, expect, it } from "vitest";
import { defaultConfig, defaultStates } from "@bw/core";
import ja from "../../i18n/locales/ja_JP.json";
import { NAMES } from "../settings/Shortcuts";
import { COLLECTIONS } from "./content";

const dictionary = ja as Record<string, string>;

describe("the tips", () => {
  it("only open surfaces that exist", () => {
    for (const tip of COLLECTIONS.flatMap((entry) => entry.tips)) {
      if (tip.open) expect(defaultStates).toHaveProperty(tip.open.flag);
    }
  });

  // Drawn from the config, so a shortcut added without a name would be a row
  // that says nothing.
  it("name every keyboard shortcut", () => {
    const keys = Object.keys(defaultConfig.keybinds).filter(
      (key) => key !== "enable",
    );
    expect(Object.keys(NAMES).sort()).toEqual(keys.sort());
  });

  it("are all translated into Japanese", () => {
    const said = COLLECTIONS.flatMap((entry) => [
      entry.title,
      entry.summary,
      ...entry.tips.flatMap((tip) => [
        tip.title,
        tip.body,
        ...(tip.open ? [tip.open.label] : []),
      ]),
    ]);
    expect(said.filter((text) => !(text in dictionary))).toEqual([]);
  });
});
