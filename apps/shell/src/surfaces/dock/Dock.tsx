// The dock.
//
// Windows already has a taskbar; this is what replaces it once the shell hides
// it (`windows.hideTaskbar`). So it behaves like one — click to raise, click
// again to minimise, right-click to pin, drag a pinned icon to move it —
// rather than like a launcher that happens to show running programs.
//
// Hiding is done by moving the whole window off the bottom of the screen and
// leaving a few pixels behind for the pointer to find. The original masks an
// input region instead, which Win32 cannot do per-region: WS_EX_TRANSPARENT is
// all-or-nothing, and a click-through window cannot notice a hover either. The
// window is cut down to the band the icons sit in instead, so the rest of the
// bottom edge belongs to the windows under it.

import { useEffect, useRef, useState } from "react";
import { Symbol, useRipple } from "../../widgets";
import { tr } from "../../i18n";
import { backend } from "../../shell/backend";
import { actions, connectDock, useShell } from "../../shell/store";
import type { DockApp } from "@bw/core";
import { reorder } from "./order";
import "./dock.css";

/** What a pinned icon needs to be dragged along the others. */
interface Drag {
  dragged: boolean;
  over: boolean;
  start: () => void;
  enter: () => void;
  drop: () => void;
  end: () => void;
}

/** One application: an icon, plus a dot per open window. */
function DockIcon({ app, drag }: { app: DockApp; drag?: Drag }) {
  const ripple = useRipple();
  const size = useShell((state) => state.config.dock.iconSize);
  const [flashed, setFlashed] = useState(false);

  const running = app.windows.length > 0;

  const activate = async () => {
    if (!running) {
      void actions.launchApp(app.executable);
      return;
    }
    // Cycle through the application's windows rather than always raising the
    // first: that is what makes a taskbar button useful for a program with
    // several windows open.
    const current = app.windows.findIndex((window) => window.active);
    const next = app.windows[(current + 1) % app.windows.length]!;

    const outcome = await actions.activateWindow(next.id, app.active);
    if (outcome === "flashed") {
      // Windows refused to move the foreground. Say so rather than looking
      // inert — the window is flashing in the taskbar and the user needs to
      // know that is where to look.
      setFlashed(true);
      window.setTimeout(() => setFlashed(false), 1200);
    }
    if (outcome === "gone") void actions.refreshDock();
  };

  return (
    <button
      type="button"
      className="bw-dock-icon"
      data-active={app.active}
      data-running={running}
      data-flashed={flashed}
      data-dragged={drag?.dragged}
      data-drop-target={drag?.over}
      draggable={drag !== undefined}
      onDragStart={(event) => {
        event.dataTransfer.effectAllowed = "move";
        drag?.start();
      }}
      onDragEnter={drag?.enter}
      onDragOver={(event) => {
        if (drag) event.preventDefault();
      }}
      onDrop={(event) => {
        event.preventDefault();
        drag?.drop();
      }}
      onDragEnd={drag?.end}
      style={{ width: size, height: size }}
      aria-label={app.name}
      title={
        app.windows.length > 1
          ? `${app.name} — ${app.windows.length}`
          : (app.windows[0]?.title ?? app.name)
      }
      onPointerDown={ripple.spawn}
      onClick={() => void activate()}
      onContextMenu={(event) => {
        event.preventDefault();
        void actions.setPinned(app.executable, !app.pinned);
      }}
    >
      {app.icon ? (
        <img src={backend().assetUrl(app.icon)} alt="" draggable={false} />
      ) : (
        <Symbol name="apps" size={Math.round(size * 0.55)} />
      )}

      {running ? (
        <span
          className="bw-dock-dot"
          data-many={app.windows.length > 1}
          aria-hidden="true"
        />
      ) : null}
      {ripple.layer}
    </button>
  );
}

export function Dock() {
  const config = useShell((state) => state.config.dock);
  const apps = useShell((state) => state.dock);
  const media = useShell((state) => state.media);
  const ready = useShell((state) => state.ready);

  const [hovered, setHovered] = useState(false);
  const [pinned, setPinned] = useState(config.pinnedOnStartup);
  const dock = useRef<HTMLDivElement>(null);
  const [dragged, setDragged] = useState<string | null>(null);
  const [over, setOver] = useState<string | null>(null);

  useEffect(() => {
    void connectDock();
  }, []);

  // The window is parked off the bottom while hidden, with only the hover strip
  // on screen, so reaching it has to bring the window back before the slide can
  // show anything — a transition inside an off-screen window is invisible. That
  // is one move per transition; the slide itself still runs in the webview.
  const revealed = pinned || !config.autoHide || hovered;

  useEffect(() => {
    if (!config.autoHide || pinned) return;
    void actions.setSurfaceRevealed("dock", hovered);
  }, [config.autoHide, pinned, hovered]);

  // The window spans the screen; only the band the icons sit in should take
  // the pointer. Measured across only: the slide moves the dock up and down,
  // never sideways, and the band keeps the window's full height either way.
  useEffect(() => {
    const element = dock.current;
    if (!element) return;
    const send = () => {
      // Room either side for the dock's shadow, which a hard edge would cut.
      const shadow = 24;
      const box = element.getBoundingClientRect();
      void actions.setDockShape(box.left - shadow, box.width + shadow * 2);
    };
    send();
    const observer = new ResizeObserver(send);
    observer.observe(element);
    window.addEventListener("resize", send);
    return () => {
      observer.disconnect();
      window.removeEventListener("resize", send);
    };
  }, [ready]);

  if (!ready) return null;

  const [pinnedApps, running] = [
    apps.filter((app) => app.pinned),
    apps.filter((app) => !app.pinned),
  ];

  return (
    <div
      className="bw-dock-root"
      data-revealed={revealed}
      onPointerEnter={() => setHovered(true)}
      onPointerLeave={() => setHovered(false)}
    >
      <div
        ref={dock}
        className="bw-dock"
        data-background={config.showBackground}
      >
        {pinnedApps.map((app) => (
          <DockIcon
            key={`${app.executable}|${app.appId}`}
            app={app}
            drag={{
              dragged: dragged === app.executable,
              over: over === app.executable && dragged !== app.executable,
              start: () => setDragged(app.executable),
              enter: () => setOver(app.executable),
              drop: () => {
                if (!dragged) return;
                const next = reorder(
                  config.pinnedApps,
                  dragged,
                  app.executable,
                );
                if (next !== config.pinnedApps)
                  void actions.setConfigValue("dock.pinnedApps", next);
              },
              end: () => {
                setDragged(null);
                setOver(null);
              },
            }}
          />
        ))}

        {pinnedApps.length > 0 && running.length > 0 ? (
          <span className="bw-dock-separator" />
        ) : null}

        {running.map((app) => (
          <DockIcon key={`${app.executable}|${app.appId}`} app={app} />
        ))}

        {config.showMedia && media?.title ? (
          <>
            <span className="bw-dock-separator" />
            <button
              type="button"
              className="bw-dock-media"
              aria-label={media.playing ? tr("Pause") : tr("Play")}
              onClick={() => void actions.mediaCommand("playPause")}
            >
              <Symbol name={media.playing ? "pause" : "play_arrow"} size={20} />
              <span className="bw-dock-media-title">{media.title}</span>
            </button>
          </>
        ) : null}

        {config.showPinButton ? (
          <>
            <span className="bw-dock-separator" />
            <button
              type="button"
              className="bw-dock-pin"
              data-on={pinned}
              aria-pressed={pinned}
              aria-label={tr("Keep the dock open")}
              onClick={() => setPinned((value) => !value)}
            >
              <Symbol
                name={pinned ? "keep" : "keep_off"}
                size={18}
                filled={pinned}
              />
            </button>
          </>
        ) : null}
      </div>
    </div>
  );
}
