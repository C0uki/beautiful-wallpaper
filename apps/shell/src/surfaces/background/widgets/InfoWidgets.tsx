// Weather, resources, media, calendar and user-card desktop widgets.
//
// Each reads from the shell store and renders nothing but tokens, so all five
// re-theme with the wallpaper.

import { Card, IconButton, ProgressRing, Symbol } from "../../../widgets";
import { tr } from "../../../i18n";
import { actions, useShell } from "../../../shell/store";

function formatBytes(bytes: number): string {
  const units = ["B", "KB", "MB", "GB", "TB"];
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }
  return `${value.toFixed(value >= 100 || unit === 0 ? 0 : 1)} ${units[unit]}`;
}

export function WeatherWidget() {
  const weather = useShell((state) => state.weather);

  if (!weather) {
    return (
      <Card style={{ minWidth: 260, color: "var(--on-surface-variant)" }}>
        {tr("Weather unavailable")}
      </Card>
    );
  }

  return (
    <Card style={{ minWidth: 300 }}>
      <div style={{ display: "flex", alignItems: "flex-start", gap: 14 }}>
        <div style={{ flex: 1 }}>
          <div style={{ display: "flex", alignItems: "baseline", gap: 8 }}>
            <span
              style={{
                fontFamily: "var(--font-title)",
                fontSize: "2.5em",
                fontWeight: 500,
                lineHeight: 1,
              }}
            >
              {Math.round(weather.temperature)}°
            </span>
            <span style={{ color: "var(--on-surface-variant)" }}>
              {weather.description}
            </span>
          </div>
          <div
            style={{
              marginTop: 4,
              color: "var(--on-surface-variant)",
              fontSize: "0.9em",
            }}
          >
            {weather.city}
          </div>
        </div>
        <div
          style={{
            display: "grid",
            placeItems: "center",
            width: 46,
            height: 46,
            borderRadius: 14,
            background: "var(--primary)",
            color: "var(--primary-on)",
          }}
        >
          <Symbol name={weather.icon} size={24} filled />
        </div>
      </div>

      <div
        style={{
          display: "flex",
          gap: 16,
          marginTop: 14,
          color: "var(--on-surface-variant)",
          fontSize: "0.86em",
        }}
      >
        <span style={{ display: "inline-flex", alignItems: "center", gap: 5 }}>
          <Symbol name="water_drop" size={15} /> {weather.humidity}%
        </span>
        <span style={{ display: "inline-flex", alignItems: "center", gap: 5 }}>
          <Symbol name="air" size={15} /> {weather.windSpeed.toFixed(1)} m/s
        </span>
        <span style={{ display: "inline-flex", alignItems: "center", gap: 5 }}>
          <Symbol name="wb_sunny" size={15} /> {weather.sunrise}
        </span>
        <span style={{ display: "inline-flex", alignItems: "center", gap: 5 }}>
          <Symbol name="bedtime" size={15} /> {weather.sunset}
        </span>
      </div>
    </Card>
  );
}

interface StatProps {
  label: string;
  percent: number;
  icon: string;
  detail?: string;
  accent: string;
}

function Stat({ label, percent, icon, detail, accent }: StatProps) {
  return (
    <Card style={{ flex: 1, minWidth: 120, padding: 14 }}>
      <div
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
        }}
      >
        <div
          style={{
            position: "relative",
            display: "grid",
            placeItems: "center",
          }}
        >
          <ProgressRing
            value={percent}
            size={44}
            thickness={4}
            color={accent}
          />
          <Symbol
            name={icon}
            size={18}
            color={accent}
            style={{ position: "absolute", inset: 0, margin: "auto" }}
          />
        </div>
        <div style={{ textAlign: "right" }}>
          <div
            style={{
              fontFamily: "var(--font-title)",
              fontSize: "1.6em",
              lineHeight: 1.1,
            }}
          >
            {Math.round(percent)}%
          </div>
          <div
            style={{ color: "var(--on-surface-variant)", fontSize: "0.82em" }}
          >
            {label}
          </div>
        </div>
      </div>
      {detail ? (
        <div
          style={{
            marginTop: 8,
            color: "var(--on-surface-variant)",
            fontSize: "0.76em",
            whiteSpace: "nowrap",
            overflow: "hidden",
            textOverflow: "ellipsis",
          }}
        >
          {detail}
        </div>
      ) : null}
    </Card>
  );
}

export function ResourcesWidget() {
  const resources = useShell((state) => state.resources);
  const showSwap = useShell((state) => state.config.resources.showSwap);
  if (!resources) return null;

  return (
    <div style={{ display: "flex", gap: 10, minWidth: 380 }}>
      <Stat
        label={tr("CPU")}
        percent={resources.cpu}
        icon="speed"
        accent="var(--primary)"
      />
      <Stat
        label={tr("RAM")}
        percent={resources.memory}
        icon="memory"
        accent="var(--secondary)"
        detail={tr(
          "%1 of %2",
          formatBytes(resources.memoryUsedBytes),
          formatBytes(resources.memoryTotalBytes),
        )}
      />
      {showSwap ? (
        <Stat
          label={tr("Swap")}
          percent={resources.swap}
          icon="storage"
          accent="var(--tertiary)"
        />
      ) : (
        <Stat
          label={tr("Disk")}
          percent={resources.disk}
          icon="storage"
          accent="var(--tertiary)"
          detail={tr(
            "%1 of %2",
            formatBytes(resources.diskUsedBytes),
            formatBytes(resources.diskTotalBytes),
          )}
        />
      )}
    </div>
  );
}

function formatTime(seconds: number): string {
  const total = Math.max(0, Math.floor(seconds));
  const minutes = Math.floor(total / 60);
  return `${minutes}:${(total % 60).toString().padStart(2, "0")}`;
}

export function MediaWidget() {
  const media = useShell((state) => state.media);

  if (!media || !media.title) {
    return (
      <Card style={{ minWidth: 340, color: "var(--on-surface-variant)" }}>
        <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
          <Symbol name="music_note" size={20} />
          {tr("Nothing playing")}
        </div>
      </Card>
    );
  }

  const progress =
    media.duration > 0 ? (media.position / media.duration) * 100 : 0;

  return (
    <Card style={{ minWidth: 360 }}>
      <div style={{ display: "flex", gap: 14 }}>
        <div
          style={{
            width: 74,
            height: 74,
            flexShrink: 0,
            borderRadius: 14,
            overflow: "hidden",
            background: "var(--layer3)",
            display: "grid",
            placeItems: "center",
          }}
        >
          {media.artwork ? (
            <img
              src={media.artwork}
              alt=""
              style={{ width: "100%", height: "100%", objectFit: "cover" }}
            />
          ) : (
            <Symbol name="album" size={30} color="var(--on-surface-variant)" />
          )}
        </div>

        <div style={{ flex: 1, minWidth: 0 }}>
          <div
            style={{
              fontFamily: "var(--font-title)",
              fontSize: "1.15em",
              fontWeight: 500,
              overflow: "hidden",
              textOverflow: "ellipsis",
              whiteSpace: "nowrap",
            }}
          >
            {media.title}
          </div>
          <div
            style={{
              color: "var(--on-surface-variant)",
              fontSize: "0.9em",
              overflow: "hidden",
              textOverflow: "ellipsis",
              whiteSpace: "nowrap",
            }}
          >
            {media.artist}
          </div>

          <div
            style={{
              display: "flex",
              alignItems: "center",
              gap: 4,
              marginTop: 8,
            }}
          >
            <IconButton
              icon="skip_previous"
              size={32}
              label={tr("Previous track")}
              onClick={() => void actions.mediaCommand("previous")}
            />
            <IconButton
              icon={media.playing ? "pause" : "play_arrow"}
              size={38}
              active
              label={media.playing ? tr("Pause") : tr("Play")}
              onClick={() => void actions.mediaCommand("playPause")}
            />
            <IconButton
              icon="skip_next"
              size={32}
              label={tr("Next track")}
              onClick={() => void actions.mediaCommand("next")}
            />
            <span
              style={{
                marginLeft: "auto",
                color: "var(--on-surface-variant)",
                fontSize: "0.8em",
                fontVariantNumeric: "tabular-nums",
              }}
            >
              {formatTime(media.position)} / {formatTime(media.duration)}
            </span>
          </div>
        </div>
      </div>

      <div
        style={{
          marginTop: 12,
          height: 4,
          borderRadius: 999,
          background: "var(--layer3)",
          overflow: "hidden",
        }}
      >
        <div
          style={{
            width: `${progress}%`,
            height: "100%",
            background: "var(--primary)",
            transition: "width 1s linear",
          }}
        />
      </div>
    </Card>
  );
}

export function CalendarWidget() {
  const weekStartsMonday = useShell(
    (state) => state.config.time.weekStartsOnMonday,
  );
  const today = new Date();
  const year = today.getFullYear();
  const month = today.getMonth();

  const firstOfMonth = new Date(year, month, 1);
  const daysInMonth = new Date(year, month + 1, 0).getDate();
  const shift = weekStartsMonday ? 1 : 0;
  const leading = (firstOfMonth.getDay() - shift + 7) % 7;

  const labels = ["Su", "Mo", "Tu", "We", "Th", "Fr", "Sa"];
  const ordered = weekStartsMonday ? [...labels.slice(1), labels[0]!] : labels;

  const cells: Array<number | null> = [
    ...Array.from({ length: leading }, () => null),
    ...Array.from({ length: daysInMonth }, (_, index) => index + 1),
  ];

  return (
    <Card style={{ minWidth: 264 }}>
      <div
        style={{
          display: "inline-block",
          padding: "4px 12px",
          borderRadius: 999,
          background: "var(--primary)",
          color: "var(--primary-on)",
          fontWeight: 600,
          marginBottom: 10,
        }}
      >
        {today.toLocaleDateString(undefined, {
          month: "long",
          year: "numeric",
        })}
      </div>

      <div
        style={{
          display: "grid",
          gridTemplateColumns: "repeat(7, 1fr)",
          gap: 2,
        }}
      >
        {ordered.map((label) => (
          <div
            key={label}
            style={{
              textAlign: "center",
              fontSize: "0.78em",
              color: "var(--on-surface-variant)",
              paddingBottom: 4,
            }}
          >
            {tr(label)}
          </div>
        ))}
        {cells.map((day, index) => {
          const isToday = day === today.getDate();
          return (
            <div
              key={index}
              style={{
                display: "grid",
                placeItems: "center",
                aspectRatio: "1",
                borderRadius: 999,
                fontSize: "0.86em",
                fontVariantNumeric: "tabular-nums",
                background: isToday ? "var(--primary)" : "transparent",
                color: isToday ? "var(--primary-on)" : "var(--on-surface)",
              }}
            >
              {day ?? ""}
            </div>
          );
        })}
      </div>
    </Card>
  );
}

export function UserCardWidget() {
  const battery = useShell((state) => state.battery);
  const workspaces = useShell((state) => state.workspaces);

  return (
    <Card style={{ minWidth: 260 }}>
      <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
        <div
          style={{
            width: 44,
            height: 44,
            borderRadius: "50%",
            background: "var(--primary)",
            color: "var(--primary-on)",
            display: "grid",
            placeItems: "center",
          }}
        >
          <Symbol name="person" size={24} filled />
        </div>
        <div style={{ flex: 1, minWidth: 0 }}>
          <div style={{ fontFamily: "var(--font-title)", fontWeight: 500 }}>
            {workspaces?.source ?? "Windows"}
          </div>
          <div
            style={{ color: "var(--on-surface-variant)", fontSize: "0.84em" }}
          >
            {battery?.present
              ? `${Math.round(battery.percent)}%${battery.charging ? " · charging" : ""}`
              : "desktop"}
          </div>
        </div>
        {battery?.present ? (
          <Symbol
            name={battery.charging ? "battery_charging_full" : "battery_full"}
            size={20}
            color="var(--on-surface-variant)"
          />
        ) : (
          <Symbol
            name="desktop_windows"
            size={20}
            color="var(--on-surface-variant)"
          />
        )}
      </div>
    </Card>
  );
}
