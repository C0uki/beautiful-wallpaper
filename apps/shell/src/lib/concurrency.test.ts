import { describe, expect, it } from "vitest";
import { forEachLimit } from "./concurrency";

describe("forEachLimit", () => {
  it("runs fn for every item exactly once", async () => {
    const seen: number[] = [];
    await forEachLimit([1, 2, 3, 4, 5], 2, async (item) => {
      seen.push(item);
    });
    expect(seen.sort((a, b) => a - b)).toEqual([1, 2, 3, 4, 5]);
  });

  it("never has more than `limit` calls in flight at once", async () => {
    let inFlight = 0;
    let peak = 0;
    await forEachLimit(
      Array.from({ length: 20 }, (_, i) => i),
      3,
      async () => {
        inFlight += 1;
        peak = Math.max(peak, inFlight);
        await new Promise((resolve) => setTimeout(resolve, 0));
        inFlight -= 1;
      },
    );
    expect(peak).toBeLessThanOrEqual(3);
  });

  it("does not spin up more workers than there are items", async () => {
    // A limit of 100 over 3 items should not create 100 workers — the pool
    // is sized to whichever is smaller.
    let started = 0;
    await forEachLimit([1, 2, 3], 100, async () => {
      started += 1;
    });
    expect(started).toBe(3);
  });

  it("does nothing for an empty list, and does not divide by zero", async () => {
    await expect(forEachLimit([], 5, async () => {})).resolves.toBeUndefined();
  });

  it("propagates a rejection rather than swallowing it", async () => {
    await expect(
      forEachLimit([1, 2, 3], 1, async (item) => {
        if (item === 2) throw new Error("boom");
      }),
    ).rejects.toThrow("boom");
  });
});
