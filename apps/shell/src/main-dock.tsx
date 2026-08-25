// Entry point for the dock.

import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { ThemeProvider } from "./design/ThemeProvider";
import { Dock } from "./surfaces/dock/Dock";
import { mountSurface } from "./shell/mount";

mountSurface("dock", (root) =>
  createRoot(root).render(
    <StrictMode>
      <ThemeProvider>
        <Dock />
      </ThemeProvider>
    </StrictMode>,
  ),
);
