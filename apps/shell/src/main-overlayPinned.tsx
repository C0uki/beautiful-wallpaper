// Entry point for the overlayPinned surface.

import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { ThemeProvider } from "./design/ThemeProvider";
import { OverlayPinned } from "./surfaces/overlay/OverlayPinned";
import { mountSurface } from "./shell/mount";

mountSurface("overlayPinned", (root) =>
  createRoot(root).render(
    <StrictMode>
      <ThemeProvider>
        <OverlayPinned />
      </ThemeProvider>
    </StrictMode>,
  ),
);
