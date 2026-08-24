// Entry point for the background surface (wallpaper + desktop widgets).

import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { ThemeProvider } from "./design/ThemeProvider";
import { Background } from "./surfaces/background/Background";
import { mountSurface } from "./shell/mount";

mountSurface("background", (root) =>
  createRoot(root).render(
    <StrictMode>
      <ThemeProvider>
        <Background />
      </ThemeProvider>
    </StrictMode>,
  ),
);
