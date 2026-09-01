import { describe, expect, it } from "vitest";
import { contrast } from "./diff";

describe("showing what a change would do", () => {
  /// The case it exists for: without this both sides read
  /// `C:/Users/you/Pictures/…` and the row shows no difference at all.
  it("drops the beginning two paths share", () => {
    const [from, to] = contrast(
      "C:/Users/you/Pictures/Wallpapers/dunes-at-dusk.jpg",
      "C:/Users/you/Pictures/Wallpapers/pine-fog.png",
    );
    expect(from).toBe("…dunes-at-dusk.jpg");
    expect(to).toBe("…pine-fog.png");
  });

  it("leaves short values exactly as they are", () => {
    expect(contrast("m3", "float")).toEqual(["m3", "float"]);
    expect(contrast("", "#ff0000")).toEqual(["", "#ff0000"]);
  });

  it("leaves long values that share nothing alone", () => {
    const from = "D:/Media/Backgrounds/a-very-long-file-name-here.png";
    const to = "C:/Users/you/Pictures/another-long-file-name.png";
    expect(contrast(from, to)).toEqual([from, to]);
  });

  /// A tail beginning in the middle of a folder name is harder to read than
  /// the whole path, so the cut lands on a separator.
  it("cuts at a boundary rather than mid-word", () => {
    const [from, to] = contrast(
      "C:/Users/you/Pictures/Wallpapers/night/one.png",
      "C:/Users/you/Pictures/Wallpapers/nightfall/two.png",
    );
    expect(from).toBe("…night/one.png");
    expect(to).toBe("…nightfall/two.png");
  });

  /// Barely-shared beginnings are not worth an ellipsis: it would cost a
  /// character and explain nothing.
  it("keeps a shared beginning too short to be worth dropping", () => {
    const from = "abc-this-is-quite-a-long-value-indeed";
    const to = "abd-this-is-quite-a-long-value-also";
    expect(contrast(from, to)).toEqual([from, to]);
  });

  it("handles one side being a prefix of the other", () => {
    const [from, to] = contrast(
      "C:/Users/you/Pictures/Wallpapers/",
      "C:/Users/you/Pictures/Wallpapers/pine-fog.png",
    );
    expect(from).toBe("…");
    expect(to).toBe("…pine-fog.png");
  });
});
