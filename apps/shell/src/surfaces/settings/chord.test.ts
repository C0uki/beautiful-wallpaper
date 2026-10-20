import { describe, expect, it } from "vitest";
import { capsOf, chordFrom } from "./chord";

const press = (
  code: string,
  key: string,
  held: Partial<Record<"ctrlKey" | "altKey" | "shiftKey" | "metaKey", boolean>>,
) => ({
  code,
  key,
  ctrlKey: false,
  altKey: false,
  shiftKey: false,
  metaKey: false,
  ...held,
});

describe("chordFrom", () => {
  it("spells a chord the way the config does", () => {
    // The shipped defaults, pressed: what is recorded must be what was there.
    expect(chordFrom(press("KeyA", "a", { ctrlKey: true, altKey: true }))).toBe(
      "Ctrl+Alt+A",
    );
    expect(
      chordFrom(press("KeyN", "N", { shiftKey: true, metaKey: true })),
    ).toBe("Shift+Super+N");
    expect(chordFrom(press("Space", " ", { altKey: true }))).toBe("Alt+Space");
    expect(
      chordFrom(press("PrintScreen", "PrintScreen", { ctrlKey: true })),
    ).toBe("Ctrl+PrintScreen");
  });

  it("keeps the key's own name, whatever Shift does to the character", () => {
    expect(chordFrom(press("Digit1", "!", { shiftKey: true }))).toBe(
      "Shift+Digit1",
    );
  });

  it("waits while only modifiers are down", () => {
    expect(chordFrom(press("ControlLeft", "Control", { ctrlKey: true }))).toBe(
      null,
    );
    expect(chordFrom(press("MetaLeft", "Meta", { metaKey: true }))).toBe(null);
  });
});

describe("capsOf", () => {
  it("names the Windows key as the key cap does", () => {
    expect(capsOf("Super+Shift+E")).toEqual(["Win", "Shift", "E"]);
    expect(capsOf("")).toEqual([]);
  });
});
