import { describe, expect, it } from "vitest";
import { createLimiter } from "./concurrency";

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
    // is the exact shape a per-batch pool cannot handle, and the reason this
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
