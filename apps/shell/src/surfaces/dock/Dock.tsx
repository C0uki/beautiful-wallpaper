// The dock.
//
// Windows already has a taskbar; this is what replaces it once the shell hides
// it (`windows.hideTaskbar`). So it behaves like one — click to raise, click
// again to minimise, right-click to pin — rather than like a launcher that
// happens to show running programs.
//
// Hiding is done by moving the whole window off the bottom of the screen and
// leaving a few pixels behind for the pointer to find. The original masks an
// input region instead, which Win32 cannot do per-region: WS_EX_TRANSPARENT is
// all-or-nothing, and a click-through window cannot notice a hover either.

import { useEffect, useState } from "react";
import { Symbol, useRipple } from "../../widgets";
import { tr } from "../../i18n";
import { backend } from "../../shell/backend";
import { actions, connectDock, useShell } from "../../shell/store";
import type { DockApp } from "@bw/core";
import "./dock.css";

/** One application: an icon, plus a dot per open window. */
function DockIcon({ app }: { app: DockApp }) {
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

  useEffect(() => {
    void connectDock();
  }, []);

  // The window is positioned by Rust; this only tells it which state to be in.
  // Keeping the class on the root means the CSS transition runs in the webview
  // rather than the window being moved every frame.
  const revealed = pinned || !config.autoHide || hovered;

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
      <div className="bw-dock" data-background={config.showBackground}>
        {pinnedApps.map((app) => (
          <DockIcon key={app.executable} app={app} />
        ))}

        {pinnedApps.length > 0 && running.length > 0 ? (
          <span className="bw-dock-separator" />
        ) : null}

        {running.map((app) => (
          <DockIcon key={app.executable} app={app} />
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
