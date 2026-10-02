import { describe, expect, it } from "vitest";
import { reorder } from "./order";

describe("rearranging the pinned applications", () => {
  const pinned = [
    "C:/Apps/Editor.exe",
    "C:\\Apps\\Browser.exe",
    "C:/Apps/Mail.exe",
  ];

  it("moves an entry forward and back, onto the one dropped on", () => {
    expect(
      reorder(pinned, "c:\\apps\\mail.exe", "c:\\apps\\editor.exe"),
    ).toEqual([
      "C:/Apps/Mail.exe",
      "C:/Apps/Editor.exe",
      "C:\\Apps\\Browser.exe",
    ]);
    expect(
      reorder(pinned, "c:\\apps\\editor.exe", "c:\\apps\\mail.exe"),
    ).toEqual([
      "C:\\Apps\\Browser.exe",
      "C:/Apps/Mail.exe",
      "C:/Apps/Editor.exe",
    ]);
  });

  it("leaves the list alone for a drop on itself or on nothing pinned", () => {
    expect(reorder(pinned, "c:\\apps\\mail.exe", "c:\\apps\\mail.exe")).toBe(
      pinned,
    );
    expect(reorder(pinned, "c:\\apps\\mail.exe", "c:\\apps\\notepad.exe")).toBe(
      pinned,
    );
  });
});
