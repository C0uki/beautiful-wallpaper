// The launcher's keywords and their meanings live on opposite sides of the
// IPC boundary, so nothing but a test holds them together.

import { describe, expect, it } from "vitest";
import { launcherActions } from "@bw/core";
import { ACTIONS } from "./actions";

describe("the launcher's `/` actions", () => {
  it("does something for every keyword the backend offers", () => {
    const missing = launcherActions.filter((keyword) => !ACTIONS[keyword]);
    expect(missing, "keywords the launcher lists but cannot run").toEqual([]);
  });

  it("offers nothing the backend will not list", () => {
    const extra = Object.keys(ACTIONS).filter(
      (keyword) => !launcherActions.includes(keyword),
    );
    expect(extra, "keywords with a handler but no way to type them").toEqual(
      [],
    );
  });

  it("describes each one, so a row says what Enter will do", () => {
    for (const keyword of launcherActions) {
      expect(ACTIONS[keyword]?.describe(), keyword).toBeTruthy();
    }
  });
});
