// Entry point for the search overlay.

import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { ThemeProvider } from "./design/ThemeProvider";
import { Overview } from "./surfaces/overview/Overview";
import { mountSurface } from "./shell/mount";

mountSurface("overview", (root) =>
  createRoot(root).render(
    <StrictMode>
      <ThemeProvider>
        <Overview />
      </ThemeProvider>
    </StrictMode>,
  ),
);
