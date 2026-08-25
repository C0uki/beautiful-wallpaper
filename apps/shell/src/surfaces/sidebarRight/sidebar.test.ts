import { describe, expect, it } from "vitest";
import { displayToPosition, positionToDisplay } from "./QuickSliders";
import { daysInMonth, leadingBlanks } from "./Calendar";
import { formatDuration } from "./Timer";
import { signalIcon } from "./dialogs/WifiDialog";

describe("the brightness slider's split travel", () => {
  it("puts real brightness above the tint band and nothing below it", () => {
    // The bottom 30% is colour temperature; the backlight is already at zero.
    expect(positionToDisplay(0).brightness).toBe(0);
    expect(positionToDisplay(15).brightness).toBe(0);
    expect(positionToDisplay(30).brightness).toBe(0);
    expect(positionToDisplay(65).brightness).toBeCloseTo(50, 5);
    expect(positionToDisplay(100).brightness).toBe(100);
  });

  it("only warms the display in the lower band", () => {
    // Above the split the tint must be neutral, or raising brightness would
    // also change the colour.
    expect(positionToDisplay(30).kelvin).toBe(6500);
    expect(positionToDisplay(100).kelvin).toBe(6500);
    expect(positionToDisplay(0).kelvin).toBe(2000);
    expect(positionToDisplay(15).kelvin).toBeGreaterThan(2000);
    expect(positionToDisplay(15).kelvin).toBeLessThan(6500);
  });

  it("round-trips a brightness back to the handle position", () => {
    for (const brightness of [0, 25, 50, 75, 100]) {
      const back = positionToDisplay(displayToPosition(brightness)).brightness;
      expect(back).toBeCloseTo(brightness, 5);
    }
  });

  it("clamps a position outside the track", () => {
    expect(positionToDisplay(-20).brightness).toBe(0);
    expect(positionToDisplay(400).brightness).toBe(100);
  });
});

describe("the calendar grid", () => {
  it("counts the days in a month, February included", () => {
    expect(daysInMonth(2026, 0)).toBe(31);
    expect(daysInMonth(2026, 1)).toBe(28);
    // 2024 is a leap year; 2100 is not, despite being divisible by four.
    expect(daysInMonth(2024, 1)).toBe(29);
    expect(daysInMonth(2100, 1)).toBe(28);
  });

  it("offsets the first of the month by the configured week start", () => {
    // 1 March 2026 is a Sunday.
    expect(leadingBlanks(2026, 2, false)).toBe(0);
    // Starting on Monday, a Sunday belongs at the end of the week, not before
    // it — the case a naive `weekday - 1` turns into -1.
    expect(leadingBlanks(2026, 2, true)).toBe(6);
  });

  it("offsets a mid-week first correctly in both modes", () => {
    // 1 April 2026 is a Wednesday.
    expect(leadingBlanks(2026, 3, false)).toBe(3);
    expect(leadingBlanks(2026, 3, true)).toBe(2);
  });
});

describe("the timer's display", () => {
  it("shows minutes and seconds, padded", () => {
    expect(formatDuration(0)).toBe("00:00");
    expect(formatDuration(9_000)).toBe("00:09");
    expect(formatDuration(65_000)).toBe("01:05");
  });

  it("grows an hours field only when there is one", () => {
    expect(formatDuration(59 * 60_000 + 59_000)).toBe("59:59");
    expect(formatDuration(3_600_000)).toBe("1:00:00");
  });

  it("never shows a negative time", () => {
    // A countdown that overshoots between ticks would otherwise read "-00:01".
    expect(formatDuration(-5_000)).toBe("00:00");
  });
});

describe("the Wi-Fi signal icon", () => {
  it("separates no signal, weak and strong", () => {
    expect(signalIcon(0)).not.toBe(signalIcon(1));
    expect(signalIcon(1)).not.toBe(signalIcon(4));
  });

  it("uses no name the font subset drops", () => {
    // Every Material Symbols name containing `_digit_` is an alias whose
    // output glyph the subsetter prunes, so it renders as the literal word.
    for (const bars of [0, 1, 2, 3, 4]) {
      expect(signalIcon(bars)).not.toContain("_digit_");
    }
  });

  it("treats an out-of-range count as full rather than blank", () => {
    expect(signalIcon(9)).toBe(signalIcon(4));
  });
});
