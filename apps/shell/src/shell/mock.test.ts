import { Command, Event, type Config } from "@bw/core";
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

  it("rejects a command it does not implement, rather than resolving undefined", async () => {
    const backend = mockBackend();
    await expect(backend.invoke("no_such_command")).rejects.toThrow(
      /no_such_command/,
    );
  });
});
