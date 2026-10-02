// The background surface: wallpaper plus desktop widgets.
//
// On Windows this window is parented to WorkerW, which places it above the
// desktop wallpaper and below the icons — the closest thing Windows has to the
// `WlrLayer.Bottom` layer the original uses.

import { useEffect, useMemo, useState } from "react";
import type { WidgetPlacement } from "@bw/core";
import { WallpaperCanvas } from "../../gl/WallpaperCanvas";
import { backend } from "../../shell/backend";
import { actions, useShell } from "../../shell/store";
import { IconButton } from "../../widgets";
import { tr } from "../../i18n";
import { WidgetCanvas, type CanvasItem } from "./WidgetCanvas";
import { ClockWidget } from "./widgets/ClockWidget";
import {
  CalendarWidget,
  MediaWidget,
  ResourcesWidget,
  UserCardWidget,
  WeatherWidget,
} from "./widgets/InfoWidgets";

/** A flat colour stands in for the image when work-safety blanking is on. */
function BlankedWallpaper() {
  return (
    <div
      style={{
        position: "absolute",
        inset: 0,
        background: "var(--layer1)",
      }}
    />
  );
}

/** Wallpapers played rather than drawn: `bw_core::wallpaper::VIDEO_EXTENSIONS`. */
const VIDEO = /\.(mp4|webm)$/i;

export function Background() {
  const config = useShell((state) => state.config);
  const wallpaper = useShell((state) => state.wallpaper);
  const editing = useShell((state) => state.states.widgetEditMode);
  const [pan, setPan] = useState({ x: 0, y: 0 });

  const background = config.background;
  const widgets = background.widgets;

  const path = wallpaper.path || background.wallpaperPath;
  // A video plays over a still frame of itself. The still is what the
  // transition runs to, what the widgets are placed against, and what is
  // left showing if the video cannot play.
  const video = VIDEO.test(path);
  const picture = video ? background.thumbnailPath : path;
  const src = useMemo(
    () => (picture ? backend().assetUrl(picture) : ""),
    [picture],
  );
  const videoSrc = useMemo(
    () => (video ? backend().assetUrl(path) : ""),
    [video, path],
  );
  const [playing, setPlaying] = useState("");

  // Parallax: nudge the wallpaper as the pointer moves, within the headroom the
  // configured zoom provides.
  useEffect(() => {
    if (!background.parallax.enable) {
      setPan({ x: 0, y: 0 });
      return;
    }
    const headroom = Math.max(0, (background.parallax.zoom - 1) * 50);
    const onMove = (event: PointerEvent) => {
      const x = (event.clientX / window.innerWidth - 0.5) * -2 * headroom;
      const y = (event.clientY / window.innerHeight - 0.5) * -2 * headroom;
      setPan({ x, y });
    };
    window.addEventListener("pointermove", onMove);
    return () => window.removeEventListener("pointermove", onMove);
  }, [background.parallax.enable, background.parallax.zoom]);

  const item = (
    id: string,
    placement: WidgetPlacement,
    node: CanvasItem["node"],
  ): CanvasItem => ({
    id,
    placement,
    configPath: `background.widgets.${id}`,
    node,
  });

  const items: CanvasItem[] = [
    item("clock", widgets.clock, <ClockWidget style={widgets.clock.style} />),
    item("media", widgets.media, <MediaWidget />),
    item("weather", widgets.weather, <WeatherWidget />),
    item("resources", widgets.resources, <ResourcesWidget />),
    item("calendar", widgets.calendar, <CalendarWidget />),
    item("userCard", widgets.userCard, <UserCardWidget />),
  ];

  return (
    <div
      style={{
        position: "relative",
        width: "100%",
        height: "100%",
        overflow: "hidden",
      }}
    >
      {wallpaper.blanked || !src ? (
        <BlankedWallpaper />
      ) : (
        <WallpaperCanvas
          src={src}
          transition={background.wallpaperAnimation}
          durationMs={background.transitionDuration}
          zoom={background.parallax.enable ? background.parallax.zoom : 1}
          panX={pan.x}
          panY={pan.y}
        />
      )}

      {/* ponytail: plays on behind full-screen programs too. Pause it there
          if the decoding ever shows up in a battery report. */}
      {videoSrc && !wallpaper.blanked ? (
        <video
          key={videoSrc}
          src={videoSrc}
          autoPlay
          muted
          loop
          playsInline
          onPlaying={() => setPlaying(videoSrc)}
          style={{
            position: "absolute",
            inset: 0,
            width: "100%",
            height: "100%",
            objectFit: "cover",
            // Faded in once it is moving, so the still underneath carries the
            // change and the first decoded frame does not pop in.
            opacity: playing === videoSrc ? 1 : 0,
            transform: `scale(${background.parallax.enable ? background.parallax.zoom : 1}) translate(${pan.x}%, ${pan.y}%)`,
            transition:
              "opacity var(--duration-slow) var(--ease-effects-default), transform var(--duration-slower) var(--ease-spatial-default)",
          }}
        />
      ) : null}

      {widgets.enable ? (
        <WidgetCanvas
          items={items}
          editing={editing}
          grid={widgets.grid}
          wallpaper={wallpaper.blanked ? "" : src}
        />
      ) : null}

      {/* The desktop's own controls: wallpaper picker, shuffle, edit mode.
          Offset past the bar when it is along the bottom edge, so the two do
          not sit on top of each other. */}
      <div
        style={{
          position: "absolute",
          right: 18,
          bottom:
            18 +
            (config.bar.enable && config.bar.bottom ? config.bar.height : 0),
          display: "flex",
          gap: 6,
          padding: 6,
          borderRadius: 999,
          background: "var(--layer2)",
          boxShadow: "0 4px 18px var(--shadow)",
        }}
      >
        <IconButton
          icon="wallpaper"
          label={tr("Wallpapers")}
          onClick={() => void actions.toggleState("wallpaperSelectorOpen")}
        />
        <IconButton
          icon="shuffle"
          label={tr("Random")}
          onClick={() => void actions.randomWallpaper()}
        />
        <IconButton
          icon={editing ? "done" : "edit"}
          label={editing ? tr("Done") : tr("Edit widgets")}
          active={editing}
          onClick={() => void actions.toggleState("widgetEditMode")}
        />
      </div>
    </div>
  );
}
