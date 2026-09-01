// Entry point for the screen's decorations.

import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { ThemeProvider } from "./design/ThemeProvider";
import { ScreenChrome } from "./surfaces/screenChrome/ScreenChrome";
import { mountSurface } from "./shell/mount";

mountSurface("screenChrome", (root) =>
  createRoot(root).render(
    <StrictMode>
      <ThemeProvider>
        <ScreenChrome />
      </ThemeProvider>
    </StrictMode>,
  ),
);
