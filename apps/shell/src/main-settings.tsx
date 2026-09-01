// Entry point for the settings screen.

import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { ThemeProvider } from "./design/ThemeProvider";
import { Settings } from "./surfaces/settings/Settings";
import { mountSurface } from "./shell/mount";

mountSurface("settings", (root) =>
  createRoot(root).render(
    <StrictMode>
      <ThemeProvider>
        <Settings />
      </ThemeProvider>
    </StrictMode>,
  ),
);
