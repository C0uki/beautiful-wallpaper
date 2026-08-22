// Entry point for the bar surface.

import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { ThemeProvider } from "./design/ThemeProvider";
import { Bar } from "./surfaces/bar/Bar";
import { mountSurface } from "./shell/mount";

mountSurface("bar", (root) =>
  createRoot(root).render(
    <StrictMode>
      <ThemeProvider>
        <Bar />
      </ThemeProvider>
    </StrictMode>,
  ),
);
