import { describe, expect, it } from "vitest";
import { formatAge, formatBytes, formatRate } from "./format";

describe("formatRate", () => {
  it("keeps bytes whole and larger units to one decimal", () => {
    expect(formatRate(0)).toBe("0B");
    expect(formatRate(512)).toBe("512B");
    expect(formatRate(1536)).toBe("1.5K");
    expect(formatRate(20 * 1024)).toBe("20K");
  });

  it("stops at the largest unit rather than running off the end", () => {
    expect(formatRate(5 * 1024 ** 4)).toMatch(/G$/);
  });

  it("treats a negative rate as zero", () => {
    // Counters can go backwards across an adapter reset; a negative bar is
    // worse than a zero one.
    expect(formatRate(-100)).toBe("0B");
  });
});

describe("formatBytes", () => {
  it("drops the decimal once the number is big enough to not need it", () => {
    expect(formatBytes(1024)).toBe("1.0 KB");
    expect(formatBytes(150 * 1024)).toBe("150 KB");
    expect(formatBytes(0)).toBe("0 B");
  });
});

describe("formatAge", () => {
  it("reads as an age, not a timestamp", () => {
    const now = 1_000_000;
    expect(formatAge(now - 10, now)).toBe("now");
    expect(formatAge(now - 120, now)).toBe("2m");
    expect(formatAge(now - 7200, now)).toBe("2h");
    expect(formatAge(now - 3 * 86_400, now)).toBe("3d");
  });

  it("does not show a future time as negative", () => {
    const now = 1_000_000;
    expect(formatAge(now + 500, now)).toBe("now");
  });
});
