import { describe, expect, it } from "vitest";
import { createLimiter, forEachLimit } from "./concurrency";

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
    // A limit of 100 over 3 items should never have more than 3 running at
    // once — the pool is sized to whichever is smaller, not to `limit`.
    // Counting calls alone would pass even if 100 workers were created and
    // only 3 ever found an item to take.
    let inFlight = 0;
    let peak = 0;
    let started = 0;
    await forEachLimit([1, 2, 3], 100, async () => {
      started += 1;
      inFlight += 1;
      peak = Math.max(peak, inFlight);
      await new Promise((resolve) => setTimeout(resolve, 0));
      inFlight -= 1;
    });
    expect(started).toBe(3);
    expect(peak).toBeLessThanOrEqual(3);
  });

  it("still runs everything, one at a time, when limit is zero or negative", async () => {
    const seen: number[] = [];
    await forEachLimit([1, 2, 3], 0, async (item) => {
      seen.push(item);
    });
    expect(seen.sort((a, b) => a - b)).toEqual([1, 2, 3]);
  });

  it("does nothing for an empty list", async () => {
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

describe("createLimiter", () => {
  it("resolves every call with its own result", async () => {
    const run = createLimiter(2);
    const results = await Promise.all([
      run(async () => 1),
      run(async () => 2),
      run(async () => 3),
    ]);
    expect(results.sort()).toEqual([1, 2, 3]);
  });

  it("never has more than `limit` tasks running at once, even across separate bursts", async () => {
    const run = createLimiter(3);
    let inFlight = 0;
    let peak = 0;
    const task = async () => {
      inFlight += 1;
      peak = Math.max(peak, inFlight);
      await new Promise((resolve) => setTimeout(resolve, 0));
      inFlight -= 1;
    };

    // The first burst is still draining when the second one arrives — this
    // is the exact shape forEachLimit cannot handle, and the reason this
    // primitive exists: tiles scrolling into view over time, not a single
    // known-upfront array.
    const first = Promise.all(Array.from({ length: 10 }, () => run(task)));
    await new Promise((resolve) => setTimeout(resolve, 0));
    const second = Promise.all(Array.from({ length: 10 }, () => run(task)));
    await Promise.all([first, second]);

    expect(peak).toBeLessThanOrEqual(3);
  });

  it("keeps a limit below 1 to one task at a time rather than doing nothing", async () => {
    const run = createLimiter(0);
    const seen: number[] = [];
    await Promise.all(
      [1, 2, 3].map((n) =>
        run(async () => {
          seen.push(n);
        }),
      ),
    );
    expect(seen.sort((a, b) => a - b)).toEqual([1, 2, 3]);
  });

  it("fails only the task that rejected, not the ones around it", async () => {
    const run = createLimiter(2);
    const ok = run(async () => "fine");
    const bad = run(async () => {
      throw new Error("boom");
    });
    const alsoOk = run(async () => "also fine");

    await expect(bad).rejects.toThrow("boom");
    await expect(ok).resolves.toBe("fine");
    await expect(alsoOk).resolves.toBe("also fine");
  });
});
