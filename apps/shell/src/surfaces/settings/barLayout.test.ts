import { describe, expect, it } from "vitest";
import { moveWidget, removeWidget, type BarLayout } from "./barLayout";

describe("moving widgets between the bar's slots", () => {
  const layout: BarLayout = {
    "bar.left": ["media"],
    "bar.center": ["workspaces", "activeWindow"],
    "bar.right": ["tray", "clock"],
  };

  it("moves a widget to another slot without leaving a copy behind", () => {
    expect(moveWidget(layout, "clock", "bar.left", 0)).toEqual({
      "bar.left": ["clock", "media"],
      "bar.center": ["workspaces", "activeWindow"],
      "bar.right": ["tray"],
    });
  });

  it("moves a widget along its own slot, either way", () => {
    // Dropped before the end: it goes last, not one short of it.
    expect(
      moveWidget(layout, "workspaces", "bar.center", 2)["bar.center"],
    ).toEqual(["activeWindow", "workspaces"]);
    expect(moveWidget(layout, "clock", "bar.right", 0)["bar.right"]).toEqual([
      "clock",
      "tray",
    ]);
  });

  it("adds a widget that was in no slot, and removes one", () => {
    expect(moveWidget(layout, "battery", "bar.right", 99)["bar.right"]).toEqual(
      ["tray", "clock", "battery"],
    );
    expect(removeWidget(layout, "tray")["bar.right"]).toEqual(["clock"]);
  });
});
