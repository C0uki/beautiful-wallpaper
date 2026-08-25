// The development harness.
//
// On Windows each surface is a separate webview positioned by the Rust side.
// There is no Windows here, so this page lays the same surfaces out side by side
// against the mock backend. It is what makes the UI reviewable — and
// screenshotable — from Linux, and it is not part of the shipped shell.

import { StrictMode, useEffect, useState } from "react";
import { createRoot } from "react-dom/client";
import { ThemeProvider } from "./design/ThemeProvider";
import { Background } from "./surfaces/background/Background";
import { Bar } from "./surfaces/bar/Bar";
import { SidebarRight } from "./surfaces/sidebarRight/SidebarRight";
import { WallpaperSelector } from "./surfaces/wallpaperSelector/WallpaperSelector";
import { actions, connect, useShell } from "./shell/store";
import { backend } from "./shell/backend";
import { Button, Segmented, Symbol } from "./widgets";
import { TRANSITION_NAMES } from "./gl/transitions";
import "./design/global.css";

type View = "desktop" | "wallpapers" | "sidebar" | "both";

function Harness() {
  const [view, setView] = useState<View>("both");
  const ready = useShell((state) => state.ready);
  const locale = useShell((state) => state.config.language.ui);
  const config = useShell((state) => state.config);
  const theme = useShell((state) => state.theme);
  const selectorOpen = useShell((state) => state.states.wallpaperSelectorOpen);
  const sidebarOpen = useShell((state) => state.states.sidebarRightOpen);

  useEffect(() => {
    void connect();
  }, []);

  if (!ready || !theme) {
    return (
      <p style={{ padding: 24, color: "#ccc" }}>
        Connecting to the mock backend…
      </p>
    );
  }

  const showSidebar = view === "sidebar" || sidebarOpen;
  const showDesktop = !showSidebar && (view === "desktop" || view === "both");
  const showSelector =
    !showSidebar && (view === "wallpapers" || view === "both" || selectorOpen);
  const barHeight = config.bar.enable ? config.bar.height : 0;

  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        height: "100%",
        background: "var(--layer0)",
      }}
    >
      <header
        style={{
          display: "flex",
          alignItems: "center",
          gap: 12,
          flexWrap: "wrap",
          padding: "10px 14px",
          borderBottom: "1px solid var(--outline-variant)",
          background: "var(--layer1)",
        }}
      >
        <strong
          style={{ display: "inline-flex", alignItems: "center", gap: 8 }}
        >
          <Symbol name="wallpaper" size={20} filled />
          beautiful-wallpaper
        </strong>
        <span
          style={{
            padding: "2px 8px",
            borderRadius: 999,
            background: "var(--tertiary)",
            color: "var(--tertiary-on)",
            fontSize: "0.74em",
            fontWeight: 700,
            letterSpacing: "0.04em",
          }}
        >
          {backend().kind.toUpperCase()} BACKEND
        </span>

        <Segmented
          options={[
            { value: "both", label: "Both", icon: "grid_view" },
            { value: "desktop", label: "Desktop", icon: "wallpaper" },
            { value: "wallpapers", label: "Picker", icon: "photo_library" },
            { value: "sidebar", label: "Sidebar", icon: "dock_to_left" },
          ]}
          value={view}
          onChange={setView}
        />

        <Segmented
          options={[
            { value: "en_US", label: "EN" },
            { value: "ja_JP", label: "日本語" },
          ]}
          value={locale}
          onChange={(next) => void actions.setConfigValue("language.ui", next)}
        />

        <Button
          icon={theme.mode === "dark" ? "dark_mode" : "light_mode"}
          onClick={() =>
            void actions.setMode(theme.mode === "dark" ? "light" : "dark")
          }
        >
          {theme.mode}
        </Button>

        <label
          style={{
            display: "inline-flex",
            alignItems: "center",
            gap: 6,
            fontSize: "0.86em",
          }}
        >
          <Symbol name="autorenew" size={16} />
          <select
            aria-label="Wallpaper transition"
            value={config.background.wallpaperAnimation}
            onChange={(event) =>
              void actions.setConfigValue(
                "background.wallpaperAnimation",
                event.target.value,
              )
            }
            style={{
              background: "var(--layer3)",
              color: "var(--on-surface)",
              border: 0,
              borderRadius: 999,
              padding: "6px 10px",
            }}
          >
            {[...TRANSITION_NAMES, "random"].map((name) => (
              <option key={name} value={name}>
                {name}
              </option>
            ))}
          </select>
        </label>

        <Button icon="shuffle" onClick={() => void actions.randomWallpaper()}>
          Next wallpaper
        </Button>

        <label
          style={{
            display: "inline-flex",
            alignItems: "center",
            gap: 6,
            fontSize: "0.86em",
          }}
        >
          <Symbol name="view_list" size={16} />
          <select
            aria-label="Bar style"
            value={config.bar.style}
            onChange={(event) =>
              void actions.setConfigValue("bar.style", event.target.value)
            }
            style={{
              background: "var(--layer3)",
              color: "var(--on-surface)",
              border: 0,
              borderRadius: 999,
              padding: "6px 10px",
            }}
          >
            {["m3", "hug", "float", "islands"].map((name) => (
              <option key={name} value={name}>
                bar: {name}
              </option>
            ))}
          </select>
        </label>

        <label
          style={{
            display: "inline-flex",
            alignItems: "center",
            gap: 6,
            fontSize: "0.86em",
          }}
        >
          <Symbol name="tune" size={16} />
          <select
            aria-label="Toggle style"
            value={config.sidebar.quickToggles.style}
            onChange={(event) =>
              void actions.setConfigValue(
                "sidebar.quickToggles.style",
                event.target.value,
              )
            }
            style={{
              background: "var(--layer3)",
              color: "var(--on-surface)",
              border: 0,
              borderRadius: 999,
              padding: "6px 10px",
            }}
          >
            {["android", "classic"].map((name) => (
              <option key={name} value={name}>
                toggles: {name}
              </option>
            ))}
          </select>
        </label>

        <Button
          icon={config.bar.bottom ? "expand_more" : "expand_less"}
          onClick={() =>
            void actions.setConfigValue("bar.bottom", !config.bar.bottom)
          }
        >
          {config.bar.bottom ? "bottom" : "top"}
        </Button>
      </header>

      <div style={{ flex: 1, display: "flex", minHeight: 0 }}>
        {showSidebar ? (
          // On Windows this is its own window pinned to the right edge; here it
          // gets a pane of roughly the same proportions.
          <div
            style={{
              width: 420,
              padding: 12,
              margin: "0 auto",
              minHeight: 0,
            }}
          >
            <SidebarRight />
          </div>
        ) : null}

        {showDesktop ? (
          <div style={{ flex: 3, minWidth: 0, position: "relative" }}>
            <Background />
            {/* On Windows the bar is its own window along a reserved edge; here
                it is overlaid on the desktop pane so both can be seen at once. */}
            {config.bar.enable ? (
              <div
                style={{
                  position: "absolute",
                  left: 0,
                  right: 0,
                  [config.bar.bottom ? "bottom" : "top"]: 0,
                  height: barHeight,
                }}
              >
                <Bar />
              </div>
            ) : null}
          </div>
        ) : null}

        {showSelector ? (
          <div
            style={{
              flex: 2,
              minWidth: 420,
              padding: 12,
              borderLeft: showDesktop
                ? "1px solid var(--outline-variant)"
                : "none",
              background: "var(--layer1)",
            }}
          >
            <WallpaperSelector />
          </div>
        ) : null}
      </div>
    </div>
  );
}

const root = document.getElementById("root");
if (!root) throw new Error("no #root element");
document.documentElement.dataset["surface"] = "devHarness";

createRoot(root).render(
  <StrictMode>
    <ThemeProvider>
      <Harness />
    </ThemeProvider>
  </StrictMode>,
);
