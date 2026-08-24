// The bar's widgets.
//
// Each is small, reads one thing from the store, and renders nothing but tokens.
// They are addressed by the names in `config.bar.left/center/right`, matching
// end4-pC's bar layout vocabulary.

import { useMemo, type ReactElement } from "react";
import { IconButton, Symbol } from "../../widgets";
import { tr } from "../../i18n";
import { actions, useShell } from "../../shell/store";
import { HoverPopup } from "./HoverPopup";
import { formatBytes, formatRate } from "../../lib/format";

export function ClockWidget() {
  const format = useShell((state) => state.config.time.format);
  const now = useShell((state) => state.now);

  const time = useMemo(() => {
    const pad = (value: number) => value.toString().padStart(2, "0");
    // The config carries a Qt-style pattern; only the handful the bar actually
    // offers are supported, and anything else falls back to the locale default.
    switch (format) {
      case "HH:mm":
        return `${pad(now.getHours())}:${pad(now.getMinutes())}`;
      case "HH:mm:ss":
        return `${pad(now.getHours())}:${pad(now.getMinutes())}:${pad(now.getSeconds())}`;
      case "hh:mm AP":
        return now.toLocaleTimeString(undefined, {
          hour: "2-digit",
          minute: "2-digit",
          hour12: true,
        });
      default:
        return now.toLocaleTimeString(undefined, { timeStyle: "short" });
    }
  }, [format, now]);

  const date = useMemo(
    () =>
      now.toLocaleDateString(undefined, {
        weekday: "short",
        day: "numeric",
        month: "short",
      }),
    [now],
  );

  return (
    <HoverPopup content={<div style={{ whiteSpace: "nowrap" }}>{date}</div>}>
      <div
        style={{
          display: "flex",
          alignItems: "center",
          gap: 6,
          padding: "0 10px",
          fontVariantNumeric: "tabular-nums",
          fontWeight: 600,
        }}
      >
        {time}
      </div>
    </HoverPopup>
  );
}

export function WorkspacesWidget() {
  const workspaces = useShell((state) => state.workspaces);

  // Without GlazeWM or komorebi there is nothing meaningful to show — Windows
  // has no public virtual-desktop enumeration — so the widget disappears rather
  // than showing a fake single workspace.
  if (!workspaces?.source || workspaces.workspaces.length === 0) return null;

  return (
    <div
      style={{
        display: "flex",
        alignItems: "center",
        gap: 4,
        padding: "0 6px",
      }}
    >
      {workspaces.workspaces.map((workspace) => (
        <span
          key={workspace.name}
          title={workspace.displayName}
          style={{
            display: "grid",
            placeItems: "center",
            minWidth: workspace.focused ? 30 : 22,
            height: 22,
            borderRadius: 999,
            fontSize: "0.8em",
            fontWeight: 700,
            background: workspace.focused
              ? "var(--primary)"
              : workspace.populated
                ? "var(--layer3)"
                : "transparent",
            color: workspace.focused
              ? "var(--primary-on)"
              : "var(--on-surface-variant)",
            border:
              workspace.populated || workspace.focused
                ? "none"
                : "1px solid var(--outline-variant)",
            transition:
              "min-width var(--duration-normal) var(--ease-spatial-default), background var(--duration-fast) var(--ease-effects-default)",
          }}
        >
          {workspace.displayName}
        </span>
      ))}
    </div>
  );
}

export function ActiveWindowWidget() {
  const active = useShell((state) => state.activeWindow);
  const title = active?.title?.trim();

  return (
    <div
      style={{
        display: "flex",
        alignItems: "center",
        gap: 6,
        padding: "0 10px",
        minWidth: 0,
        maxWidth: 420,
        color: title ? "var(--on-surface)" : "var(--on-surface-variant)",
      }}
    >
      <Symbol name={title ? "desktop_windows" : "wallpaper"} size={16} />
      <span
        style={{
          overflow: "hidden",
          textOverflow: "ellipsis",
          whiteSpace: "nowrap",
          fontSize: "0.9em",
        }}
      >
        {title || tr("Desktop")}
      </span>
    </div>
  );
}

export function ResourcesWidget() {
  const resources = useShell((state) => state.resources);
  if (!resources) return null;

  const pill = (icon: string, value: number, label: string) => (
    <span
      style={{ display: "inline-flex", alignItems: "center", gap: 4 }}
      title={`${label} ${Math.round(value)}%`}
    >
      <Symbol name={icon} size={15} />
      <span style={{ fontVariantNumeric: "tabular-nums", fontSize: "0.85em" }}>
        {Math.round(value)}%
      </span>
    </span>
  );

  return (
    <HoverPopup
      content={
        <div style={{ display: "grid", gap: 4, whiteSpace: "nowrap" }}>
          <div>
            {tr("RAM")} — {formatBytes(resources.memoryUsedBytes)} /{" "}
            {formatBytes(resources.memoryTotalBytes)}
          </div>
          <div>
            {tr("Disk")} — {formatBytes(resources.diskUsedBytes)} /{" "}
            {formatBytes(resources.diskTotalBytes)}
          </div>
        </div>
      }
    >
      <div
        style={{
          display: "flex",
          alignItems: "center",
          gap: 10,
          padding: "0 10px",
        }}
      >
        {pill("speed", resources.cpu, tr("CPU"))}
        {pill("memory", resources.memory, tr("RAM"))}
        {pill("storage", resources.disk, tr("Disk"))}
      </div>
    </HoverPopup>
  );
}

export function NetworkWidget() {
  const network = useShell((state) => state.network);
  if (!network) return null;

  return (
    <HoverPopup
      content={
        <div style={{ display: "grid", gap: 4, whiteSpace: "nowrap" }}>
          <div>↓ {formatBytes(network.totalReceived)}</div>
          <div>↑ {formatBytes(network.totalSent)}</div>
        </div>
      }
    >
      <div
        style={{
          display: "flex",
          alignItems: "center",
          gap: 8,
          padding: "0 10px",
          fontVariantNumeric: "tabular-nums",
          fontSize: "0.82em",
        }}
      >
        <span style={{ display: "inline-flex", alignItems: "center", gap: 3 }}>
          <Symbol name="arrow_downward" size={14} />
          {formatRate(network.down)}
        </span>
        <span style={{ display: "inline-flex", alignItems: "center", gap: 3 }}>
          <Symbol name="arrow_upward" size={14} />
          {formatRate(network.up)}
        </span>
      </div>
    </HoverPopup>
  );
}

export function BatteryWidget() {
  const battery = useShell((state) => state.battery);
  // A desktop has no battery, and an empty slot in the bar is better than a
  // permanently full icon.
  if (!battery?.present) return null;

  const hours = battery.secondsRemaining
    ? Math.floor(battery.secondsRemaining / 3600)
    : null;
  const minutes = battery.secondsRemaining
    ? Math.floor((battery.secondsRemaining % 3600) / 60)
    : null;

  const low = battery.percent <= 20 && !battery.charging;

  return (
    <HoverPopup
      content={
        <div style={{ whiteSpace: "nowrap" }}>
          {hours !== null ? `${hours}h ${minutes}m remaining` : "estimating…"}
        </div>
      }
    >
      <div
        style={{
          display: "flex",
          alignItems: "center",
          gap: 5,
          padding: "0 10px",
          color: low ? "var(--error)" : "var(--on-surface)",
        }}
      >
        <Symbol
          name={battery.charging ? "battery_charging_full" : "battery_full"}
          size={17}
          filled={battery.charging}
        />
        <span
          style={{ fontVariantNumeric: "tabular-nums", fontSize: "0.85em" }}
        >
          {Math.round(battery.percent)}%
        </span>
      </div>
    </HoverPopup>
  );
}

export function WeatherWidget() {
  const weather = useShell((state) => state.weather);
  if (!weather) return null;

  return (
    <HoverPopup
      content={
        <div style={{ whiteSpace: "nowrap" }}>
          {weather.city} — {weather.description}
        </div>
      }
    >
      <div
        style={{
          display: "flex",
          alignItems: "center",
          gap: 5,
          padding: "0 10px",
        }}
      >
        <Symbol name={weather.icon} size={17} filled />
        <span
          style={{ fontVariantNumeric: "tabular-nums", fontSize: "0.85em" }}
        >
          {Math.round(weather.temperature)}°
        </span>
      </div>
    </HoverPopup>
  );
}

export function MediaWidget() {
  const media = useShell((state) => state.media);
  if (!media?.title) return null;

  return (
    <div
      style={{
        display: "flex",
        alignItems: "center",
        gap: 4,
        padding: "0 6px",
        minWidth: 0,
      }}
    >
      <IconButton
        icon={media.playing ? "pause" : "play_arrow"}
        size={28}
        label={media.playing ? tr("Pause") : tr("Play")}
        onClick={() => void actions.mediaCommand("playPause")}
      />
      <span
        style={{
          maxWidth: 220,
          overflow: "hidden",
          textOverflow: "ellipsis",
          whiteSpace: "nowrap",
          fontSize: "0.85em",
        }}
        title={`${media.title} — ${media.artist}`}
      >
        {media.title}
      </span>
      <IconButton
        icon="skip_next"
        size={28}
        label={tr("Next track")}
        onClick={() => void actions.mediaCommand("next")}
      />
    </div>
  );
}

export function TrayWidget() {
  const tray = useShell((state) => state.tray);
  const visible = tray.filter((icon) => !icon.hidden);
  if (visible.length === 0) return null;

  // Windows does not publish the icon bitmaps to other processes in any
  // supported way, so each entry is shown as a dot until the icon extraction in
  // the roadmap lands. The count is still useful, and the tooltip identifies
  // the owner.
  return (
    <div
      style={{
        display: "flex",
        alignItems: "center",
        gap: 6,
        padding: "0 10px",
      }}
    >
      {visible.map((icon) => (
        <span
          key={`${icon.window}-${icon.id}`}
          title={icon.tooltip || icon.window}
          style={{
            width: 8,
            height: 8,
            borderRadius: "50%",
            background: "var(--on-surface-variant)",
          }}
        />
      ))}
    </div>
  );
}

export function UtilButtonsWidget() {
  const states = useShell((state) => state.states);

  return (
    <div
      style={{
        display: "flex",
        alignItems: "center",
        gap: 2,
        padding: "0 4px",
      }}
    >
      <IconButton
        icon="wallpaper"
        size={30}
        label={tr("Wallpapers")}
        active={states.wallpaperSelectorOpen}
        onClick={() => void actions.toggleState("wallpaperSelectorOpen")}
      />
      <IconButton
        icon="shuffle"
        size={30}
        label={tr("Random")}
        onClick={() => void actions.randomWallpaper()}
      />
      <IconButton
        icon="tune"
        size={30}
        label={tr("Settings")}
        active={states.settingsOpen}
        onClick={() => void actions.toggleState("settingsOpen")}
      />
    </div>
  );
}

/** Everything the bar layout can name. */
export const BAR_WIDGETS: Record<string, () => ReactElement | null> = {
  clock: ClockWidget,
  workspaces: WorkspacesWidget,
  activeWindow: ActiveWindowWidget,
  resources: ResourcesWidget,
  network: NetworkWidget,
  battery: BatteryWidget,
  weather: WeatherWidget,
  media: MediaWidget,
  tray: TrayWidget,
  utilButtons: UtilButtonsWidget,
};
