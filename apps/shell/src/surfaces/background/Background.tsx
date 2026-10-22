// The background surface: wallpaper plus desktop widgets.
//
// On Windows this window is parented to WorkerW, which places it above the
// desktop wallpaper and below the icons — the closest thing Windows has to the
// `WlrLayer.Bottom` layer the original uses. It spans every monitor, and the
// wallpaper is drawn on each; the widgets and the desktop's controls are on
// the primary one.

import { useEffect, useMemo, useRef, useState } from "react";
import { Command, type MonitorInfo, type WidgetPlacement } from "@bw/core";
import { WallpaperCanvas } from "../../gl/WallpaperCanvas";
import { backend } from "../../shell/backend";
import { actions, useShell } from "../../shell/store";
import { IconButton } from "../../widgets";
import { tr } from "../../i18n";
import { WidgetCanvas, type CanvasItem } from "./WidgetCanvas";
import { monitorParts } from "./screens";
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

  // The monitors, asked again whenever the window is resized: it is moved
  // and resized over the monitors as they now are each time one comes or goes.
  const [monitors, setMonitors] = useState<MonitorInfo[]>([]);
  const [size, setSize] = useState({
    width: window.innerWidth,
    height: window.innerHeight,
  });
  useEffect(() => {
    const look = () => {
      setSize({ width: window.innerWidth, height: window.innerHeight });
      void backend()
        .invoke<MonitorInfo[]>(Command.GetMonitors)
        .then(setMonitors)
        .catch(() => {});
    };
    look();
    window.addEventListener("resize", look);
    return () => window.removeEventListener("resize", look);
  }, []);
  const parts = monitorParts(monitors, size.width, size.height);

  // Nobody sees the wallpaper behind a full-screen program, and decoding a
  // video for nobody is a battery spent on nothing.
  // ponytail: one decoder per monitor for a video wallpaper; one video drawn
  // into every monitor's canvas if that shows up in the battery.
  const rootRef = useRef<HTMLDivElement>(null);
  const covered = useShell((state) => state.activeWindow?.fullscreen ?? false);
  useEffect(() => {
    for (const video of rootRef.current?.querySelectorAll("video") ?? []) {
      if (covered) video.pause();
      else void video.play().catch(() => {});
    }
  }, [covered, videoSrc, parts.length]);

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

  const drawn = (
    <>
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
            transform: `translate(${pan.x}%, ${pan.y}%) scale(${background.parallax.enable ? background.parallax.zoom : 1})`,
            transition:
              "opacity var(--duration-slow) var(--ease-effects-default), transform var(--duration-slower) var(--ease-spatial-default)",
          }}
        />
      ) : null}
    </>
  );

  const furniture = (
    <>
      {widgets.enable ? (
        <WidgetCanvas
          items={items}
          editing={editing}
          grid={widgets.grid}
          wallpaper={wallpaper.blanked ? "" : src}
          zoom={background.parallax.enable ? background.parallax.zoom : 1}
        />
      ) : null}

      {/* The desktop's own controls: wallpaper picker, shuffle, edit mode. */}
      <div
        style={{
          position: "absolute",
          right: 18,
          bottom: 18,
          display: "flex",
          gap: 6,
          padding: 6,
          borderRadius: 999,
          background: "var(--layer2)",
          boxShadow: "0 4px 18px var(--shadow)",
        }}
      >
        <IconButton
          icon="photo_library"
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
    </>
  );

  return (
    <div
      ref={rootRef}
      style={{
        position: "relative",
        width: "100%",
        height: "100%",
        overflow: "hidden",
      }}
    >
      {parts.map((part, index) => (
        <div
          key={index}
          style={{
            position: "absolute",
            left: part.left,
            top: part.top,
            width: part.width,
            height: part.height,
            overflow: "hidden",
          }}
        >
          {drawn}
          {part.primary ? furniture : null}
        </div>
      ))}
    </div>
  );
}
