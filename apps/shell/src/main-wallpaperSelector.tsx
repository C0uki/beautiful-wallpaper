// Entry point for the wallpaper picker surface.

import { WallpaperSelector } from "./surfaces/wallpaperSelector/WallpaperSelector";
import { mountSurface } from "./shell/mount";

mountSurface(
  "wallpaperSelector",
  <div style={{ width: "100%", height: "100%", padding: 12 }}>
    <WallpaperSelector />
  </div>,
);
