import { Command, Event, type Config, type Persistent } from "@bw/core";
import { describe, expect, it } from "vitest";
import { mockBackend } from "./mock";

describe("the mock backend", () => {
  it("hands back a fresh config object on every edit", async () => {
    const backend = mockBackend();
    const before = await backend.invoke<Config>(Command.GetConfig);
    const after = await backend.invoke<Config>(Command.SetConfigValue, {
      path: "bar.style",
      value: "islands",
    });

    expect(after.bar.style).toBe("islands");
    // Identity matters as much as the value: the store selects object slices,
    // and a mutated-in-place config would never look changed to React.
    expect(after).not.toBe(before);
    expect(after.bar).not.toBe(before.bar);
    expect(before.bar.style).toBe("m3");
  });

  it("emits the new config to listeners", async () => {
    const backend = mockBackend();
    const seen: Config[] = [];
    await backend.listen<Config>(Event.ConfigChanged, (config) =>
      seen.push(config),
    );

    await backend.invoke(Command.SetConfigValue, {
      path: "bar.height",
      value: 52,
    });

    // The first delivery is the current state on subscribe; the edit follows.
    await new Promise((resolve) => queueMicrotask(() => resolve(undefined)));
    expect(seen.at(-1)?.bar.height).toBe(52);
  });

  it("applying a wallpaper re-themes and reports the new path", async () => {
    const backend = mockBackend();
    const before = await backend.invoke<Config>(Command.GetConfig);
    const entries = await backend.invoke<
      Array<{ path: string; isDirectory: boolean }>
    >(Command.ListWallpapers);
    const other = entries.find(
      (entry) =>
        !entry.isDirectory && entry.path !== before.background.wallpaperPath,
    );
    expect(other).toBeDefined();

    await backend.invoke(Command.ApplyWallpaper, { path: other!.path });
    const after = await backend.invoke<Config>(Command.GetConfig);

    expect(after.background.wallpaperPath).toBe(other!.path);
    expect(after).not.toBe(before);
  });

  it("switching mode rebuilds the palette rather than relabelling it", async () => {
    const backend = mockBackend();
    const dark = await backend.invoke<{
      mode: string;
      colors: Record<string, string>;
    }>(Command.GetTheme);
    await backend.invoke(Command.SetMode, { mode: "light" });
    const light = await backend.invoke<{
      mode: string;
      colors: Record<string, string>;
    }>(Command.GetTheme);

    expect(dark.mode).toBe("dark");
    expect(light.mode).toBe("light");
    expect(light.colors["surface"]).not.toBe(dark.colors["surface"]);
  });

  it("dismissing a notification removes it and tells listeners", async () => {
    const backend = mockBackend();
    const before = await backend.invoke<Array<{ id: number }>>(
      Command.GetNotifications,
    );
    expect(before.length).toBeGreaterThan(0);

    const seen: Array<Array<{ id: number }>> = [];
    await backend.listen<Array<{ id: number }>>(Event.Notifications, (list) =>
      seen.push(list),
    );

    await backend.invoke(Command.DismissNotification, { id: before[0]!.id });
    const after = await backend.invoke<Array<{ id: number }>>(
      Command.GetNotifications,
    );

    expect(after.map((n) => n.id)).not.toContain(before[0]!.id);
    expect(seen.at(-1)?.length).toBe(before.length - 1);
  });

  it("clearing empties the history", async () => {
    const backend = mockBackend();
    await backend.invoke(Command.ClearNotifications);
    const after = await backend.invoke<unknown[]>(Command.GetNotifications);
    expect(after).toEqual([]);
  });

  it("serves a volume reading in the range the readout draws", async () => {
    const backend = mockBackend();
    const volume = await backend.invoke<{ percent: number; muted: boolean }>(
      Command.GetVolume,
    );
    expect(volume.percent).toBeGreaterThanOrEqual(0);
    expect(volume.percent).toBeLessThanOrEqual(100);
    expect(typeof volume.muted).toBe("boolean");
  });

  it("reports a brightness level and moves it in steps", async () => {
    const backend = mockBackend();
    const before = await backend.invoke<{
      percent: number | null;
      supported: boolean;
    }>(Command.GetBrightness);
    expect(before.supported).toBe(true);
    expect(before.percent).not.toBeNull();

    await backend.invoke(Command.StepBrightness, { up: true });
    const after = await backend.invoke<{ percent: number | null }>(
      Command.GetBrightness,
    );
    expect(after.percent).toBe(before.percent! + 5);
  });

  it("clamps brightness rather than running past the ends of the slider", async () => {
    const backend = mockBackend();
    await backend.invoke(Command.SetBrightness, { percent: 400 });
    const high = await backend.invoke<{ percent: number }>(
      Command.GetBrightness,
    );
    expect(high.percent).toBe(100);

    // And stepping up from the top stays there instead of wrapping.
    await backend.invoke(Command.StepBrightness, { up: true });
    const stillHigh = await backend.invoke<{ percent: number }>(
      Command.GetBrightness,
    );
    expect(stillHigh.percent).toBe(100);
  });

  it("the night light toggle survives in the config, not just in memory", async () => {
    const backend = mockBackend();
    const updated = await backend.invoke<Config>(Command.SetNightLight, {
      enable: true,
    });
    expect(updated.sidebar.nightLight.enable).toBe(true);

    const reread = await backend.invoke<Config>(Command.GetConfig);
    expect(reread.sidebar.nightLight.enable).toBe(true);
  });

  it("moving one application's volume leaves the others alone", async () => {
    const backend = mockBackend();
    const before = await backend.invoke<Array<{ id: string; percent: number }>>(
      Command.GetAudioSessions,
    );
    expect(before.length).toBeGreaterThan(1);

    await backend.invoke(Command.SetSessionVolume, {
      id: before[0]!.id,
      percent: 10,
    });
    const after = await backend.invoke<Array<{ id: string; percent: number }>>(
      Command.GetAudioSessions,
    );

    expect(after[0]!.percent).toBe(10);
    expect(after[1]!.percent).toBe(before[1]!.percent);
    // Fresh objects, or a store selecting the array would never see the change.
    expect(after).not.toBe(before);
    expect(after[0]).not.toBe(before[0]);
  });

  it("the to-do list adds, finishes and clears", async () => {
    const backend = mockBackend();
    const added = await backend.invoke<Array<{ id: number; done: boolean }>>(
      Command.AddTodo,
      { content: "Something new" },
    );
    const newest = added.at(-1)!;
    expect(newest.done).toBe(false);

    await backend.invoke(Command.SetTodoDone, { id: newest.id, done: true });
    const cleared = await backend.invoke<Array<{ id: number }>>(
      Command.ClearDoneTodos,
    );
    expect(cleared.map((todo) => todo.id)).not.toContain(newest.id);
  });

  it("a blank task is refused rather than added", async () => {
    const backend = mockBackend();
    const before = await backend.invoke<unknown[]>(Command.GetTodos);
    const after = await backend.invoke<unknown[]>(Command.AddTodo, {
      content: "   ",
    });
    expect(after.length).toBe(before.length);
  });

  it("persistent state takes a dotted edit and keeps a fresh object", async () => {
    const backend = mockBackend();
    const before = await backend.invoke<Persistent>(Command.GetPersistent);
    const after = await backend.invoke<Persistent>(Command.SetPersistentValue, {
      path: "sidebar.bottomGroup.tab",
      value: 2,
    });

    expect(after.sidebar.bottomGroup.tab).toBe(2);
    expect(after.sidebar).not.toBe(before.sidebar);
  });

  it("a wrong Wi-Fi password reports back as such, not as a generic failure", async () => {
    const backend = mockBackend();
    expect(
      await backend.invoke(Command.ConnectWifi, {
        ssid: "Kingfisher",
        password: "wrong",
      }),
    ).toBe("badPassword");
    expect(
      await backend.invoke(Command.ConnectWifi, {
        ssid: "Kingfisher",
        password: "right",
      }),
    ).toBe("connected");
  });

  it("clicking a dock icon makes exactly one window active", async () => {
    const backend = mockBackend();
    const before = await backend.invoke<
      Array<{ windows: Array<{ id: string; active: boolean }> }>
    >(Command.GetDockItems);
    const target = before.flatMap((app) => app.windows).find((w) => !w.active);
    expect(target).toBeDefined();

    expect(
      await backend.invoke(Command.ActivateWindow, {
        id: target!.id,
        minimiseIfActive: false,
      }),
    ).toBe("activated");

    const after = await backend.invoke<
      Array<{ windows: Array<{ id: string; active: boolean }> }>
    >(Command.GetDockItems);
    const active = after.flatMap((app) => app.windows).filter((w) => w.active);
    expect(active.map((w) => w.id)).toEqual([target!.id]);
  });

  it("clicking the application you are already in minimises it", async () => {
    const backend = mockBackend();
    const items = await backend.invoke<
      Array<{ windows: Array<{ id: string; active: boolean }> }>
    >(Command.GetDockItems);
    const active = items.flatMap((app) => app.windows).find((w) => w.active);

    expect(
      await backend.invoke(Command.ActivateWindow, {
        id: active!.id,
        minimiseIfActive: true,
      }),
    ).toBe("minimised");
  });

  it("unpinning something that is not running takes it off the dock", async () => {
    const backend = mockBackend();
    const before = await backend.invoke<
      Array<{ executable: string; pinned: boolean; windows: unknown[] }>
    >(Command.GetDockItems);
    const idle = before.find((app) => app.pinned && app.windows.length === 0);
    expect(idle).toBeDefined();

    await backend.invoke(Command.SetPinned, {
      path: idle!.executable,
      pinned: false,
    });
    const after = await backend.invoke<Array<{ executable: string }>>(
      Command.GetDockItems,
    );
    expect(after.map((app) => app.executable)).not.toContain(idle!.executable);
  });

  it("unpinning something that is running keeps its icon", async () => {
    const backend = mockBackend();
    const before = await backend.invoke<
      Array<{ executable: string; pinned: boolean; windows: unknown[] }>
    >(Command.GetDockItems);
    const busy = before.find((app) => app.pinned && app.windows.length > 0);

    await backend.invoke(Command.SetPinned, {
      path: busy!.executable,
      pinned: false,
    });
    const after = await backend.invoke<
      Array<{ executable: string; pinned: boolean }>
    >(Command.GetDockItems);
    const still = after.find((app) => app.executable === busy!.executable);
    expect(still).toBeDefined();
    expect(still!.pinned).toBe(false);
  });

  it("the translator reports a missing key rather than an empty translation", async () => {
    const backend = mockBackend();
    await backend.invoke(Command.SetAiKey, { key: "" });
    expect(await backend.invoke<boolean>(Command.HasAiKey)).toBe(false);

    const outcome = await backend.invoke<{
      text: string;
      error: string | null;
    }>(Command.Translate, { text: "Good morning", from: "auto", to: "ja" });
    // An empty string with no error would look like the translator had eaten
    // the input; the UI needs to know it should point at the settings.
    expect(outcome.error).toBe("noKey");
  });

  it("translating nothing is not an error", async () => {
    const backend = mockBackend();
    const outcome = await backend.invoke<{
      text: string;
      error: string | null;
    }>(Command.Translate, { text: "   ", from: "auto", to: "ja" });
    expect(outcome.error).toBeNull();
    expect(outcome.text).toBe("");
  });

  it("rejects a command it does not implement, rather than resolving undefined", async () => {
    const backend = mockBackend();
    await expect(backend.invoke("no_such_command")).rejects.toThrow(
      /no_such_command/,
    );
  });
});
