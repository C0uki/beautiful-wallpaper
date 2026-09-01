import { describe, expect, it } from "vitest";
import { resumeAt, STEPS } from "./steps";

describe("the first-run steps", () => {
  it("gives every step a distinct id and something to say", () => {
    const ids = STEPS.map((step) => step.id);
    expect(new Set(ids).size).toBe(ids.length);
    for (const step of STEPS) {
      expect(step.title(), step.id).toBeTruthy();
      expect(step.blurb(), step.id).toBeTruthy();
      expect(step.icon, step.id).toBeTruthy();
    }
  });

  /// A state file from a build with more steps names one that is not here, and
  /// the screen would come up blank on a machine that had merely downgraded.
  it("resumes on a step that exists, whatever the state file says", () => {
    expect(resumeAt(0)).toBe(0);
    expect(resumeAt(2)).toBe(2);
    expect(resumeAt(STEPS.length + 5)).toBe(STEPS.length - 1);
    expect(resumeAt(-1)).toBe(0);
    expect(resumeAt(Number.NaN)).toBe(0);
    expect(resumeAt(1.7)).toBe(1);
  });
});
