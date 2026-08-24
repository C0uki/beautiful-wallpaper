// Entry point for the wallpaper picker surface.

import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { ThemeProvider } from "./design/ThemeProvider";
import { WallpaperSelector } from "./surfaces/wallpaperSelector/WallpaperSelector";
import { mountSurface } from "./shell/mount";

mountSurface("wallpaperSelector", (root) =>
  createRoot(root).render(
    <StrictMode>
      <ThemeProvider>
        <div style={{ width: "100%", height: "100%", padding: 12 }}>
          <WallpaperSelector />
        </div>
      </ThemeProvider>
    </StrictMode>,
  ),
);
