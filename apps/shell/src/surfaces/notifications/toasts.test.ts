import { describe, expect, it } from "vitest";
import { toastLife } from "./Toasts";

/** A notification posted `secondsAgo` seconds before `now`. */
function posted(secondsAgo: number, now: number) {
  return { time: (now - secondsAgo * 1000) / 1000 };
}

const TIMEOUT = 7000;
const NOW = 1_800_000_000_000;

describe("toastLife", () => {
  it("gives a notification that just arrived its whole welcome", () => {
    expect(toastLife(posted(0, NOW), TIMEOUT, NOW)).toBe(TIMEOUT);
  });

  it("gives one that arrived before this page only what is left of it", () => {
    const left = toastLife(posted(2, NOW), TIMEOUT, NOW);
    expect(left).toBe(TIMEOUT - 2000);
    expect(left).toBeGreaterThan(0);
  });

  it("keeps the history out of the toasts", () => {
    // The store is persisted, so at startup the list holds every notification
    // of every past session. None of them is news.
    expect(toastLife(posted(8, NOW), TIMEOUT, NOW)).toBeLessThanOrEqual(0);
    expect(toastLife(posted(86_400, NOW), TIMEOUT, NOW)).toBeLessThanOrEqual(0);
  });

  it("does not spare an old one for being urgent", () => {
    // A `critical` toast is never given an expiry timer, so one revived from
    // the history would stay on screen for the life of the shell — and its
    // window, which is a quarter of the screen and not click-through, would
    // take every click in that rectangle with it.
    expect(toastLife(posted(3600, NOW), TIMEOUT, NOW)).toBeLessThanOrEqual(0);
  });
});
