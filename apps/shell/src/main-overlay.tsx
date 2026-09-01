// Entry point for the overlay surface.

import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { ThemeProvider } from "./design/ThemeProvider";
import { Overlay } from "./surfaces/overlay/Overlay";
import { mountSurface } from "./shell/mount";

mountSurface("overlay", (root) =>
  createRoot(root).render(
    <StrictMode>
      <ThemeProvider>
        <Overlay />
      </ThemeProvider>
    </StrictMode>,
  ),
);
