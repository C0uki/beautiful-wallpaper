// Entry point for the hot corners.

import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { ThemeProvider } from "./design/ThemeProvider";
import { HotCorners } from "./surfaces/hotCorners/HotCorners";
import { mountSurface } from "./shell/mount";

mountSurface("hotCorners", (root) =>
  createRoot(root).render(
    <StrictMode>
      <ThemeProvider>
        <HotCorners />
      </ThemeProvider>
    </StrictMode>,
  ),
);
