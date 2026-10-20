import { Command, Event } from "@bw/core";
import { expect, it, vi } from "vitest";

// A backend that records the chat commands it is sent, and answers the rest
// as the mock does.
const backend = vi.hoisted(() => ({
  calls: [] as Array<[string, Record<string, unknown> | undefined]>,
  listeners: new Map<string, (payload: unknown) => void>(),
}));

vi.mock("./backend", () => ({
  backend: () => ({
    kind: "mock",
    assetUrl: (path: string) => path,
    async listen(event: string, handler: (payload: unknown) => void) {
      backend.listeners.set(event, handler);
      return () => {};
    },
    async invoke(command: string, args?: Record<string, unknown>) {
      if (command === "send_chat" || command === "retry_chat") {
        backend.calls.push([command, args]);
        return command === "retry_chat" ? [] : undefined;
      }
      // Everything else — what connecting reads — as the mock answers it.
      const { mockBackend } = await import("./mock");
      return mockBackend().invoke(command, args);
    },
  }),
}));

it("retrying sends the failed question again, with its files", async () => {
  const { actions, useShell } = await import("./store");
  await actions.sendChat("What is this?", ["C:\\diagram.png"]);

  const turn = { thinking: "", searches: [], sources: [], answeredBy: "" };
  useShell.setState({
    chat: [
      {
        ...turn,
        id: 1,
        role: "user",
        content: "What is this?",
        attachments: ["diagram.png"],
        time: 0,
      },
      {
        ...turn,
        id: 2,
        role: "assistant",
        content: "",
        attachments: [],
        time: 0,
      },
    ],
  });
  await actions.retryChat();

  expect(backend.calls.map(([command]) => command)).toEqual([
    Command.SendChat,
    Command.RetryChat,
    Command.SendChat,
  ]);
  expect(backend.calls.at(-1)![1]).toEqual({
    text: "What is this?",
    attachments: ["C:\\diagram.png"],
  });
});

it("a failed reply keeps why, until the next question", async () => {
  const { actions, connectSidebarLeft, useShell } = await import("./store");
  void connectSidebarLeft();
  await vi.waitFor(() =>
    expect(backend.listeners.has(Event.ChatEvent)).toBe(true),
  );

  backend.listeners.get(Event.ChatEvent)!({
    kind: "failed",
    value: "rateLimited",
  });
  expect(useShell.getState().chatError).toBe("rateLimited");

  await actions.sendChat("Again?");
  expect(useShell.getState().chatError).toBeNull();
});
