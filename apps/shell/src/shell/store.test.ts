import { Event, type GlobalStates } from "@bw/core";
import { expect, it, vi } from "vitest";

// A backend that answers the first snapshot of the states late, after a
// change made since it was taken — what the shell does while it is starting.
const backend = vi.hoisted(() => {
  const listeners = new Map<string, (payload: unknown) => void>();
  let release = () => {};
  const held = new Promise<void>((resolve) => (release = resolve));
  return { listeners, held, release: () => release(), asked: false };
});

vi.mock("./backend", () => ({
  backend: () => ({
    kind: "mock",
    assetUrl: (path: string) => path,
    async listen(event: string, handler: (payload: unknown) => void) {
      backend.listeners.set(event, handler);
      return () => {};
    },
    async invoke(command: string) {
      const { mockBackend } = await import("./mock");
      const answer = await mockBackend().invoke(command);
      if (command === "get_states") {
        backend.asked = true;
        await backend.held;
      }
      return answer;
    },
  }),
}));

it("keeps a change that overtook the first snapshot", async () => {
  const { connect, useShell } = await import("./store");
  const connecting = connect();
  await vi.waitFor(() => expect(backend.asked).toBe(true));

  const opened = { ...useShell.getState().states, shelfOpen: true };
  backend.listeners.get(Event.StateChanged)?.(opened satisfies GlobalStates);
  backend.release();
  await connecting;

  expect(useShell.getState().ready).toBe(true);
  expect(useShell.getState().states.shelfOpen).toBe(true);
});

it("fills the state from the readings a page asks for when it loads", async () => {
  const { latestReadings } = await import("./store");
  expect(
    latestReadings({
      [Event.Weather]: { city: "Komaki" },
      [Event.Tray]: [],
      "bw://unknown": 1,
    }),
  ).toEqual({ weather: { city: "Komaki" }, tray: [] });
  expect(latestReadings(null)).toEqual({});
});
