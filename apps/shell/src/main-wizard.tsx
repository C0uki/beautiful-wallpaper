// Entry point for the first-run screen.

import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { ThemeProvider } from "./design/ThemeProvider";
import { Wizard } from "./surfaces/wizard/Wizard";
import { mountSurface } from "./shell/mount";

mountSurface("wizard", (root) =>
  createRoot(root).render(
    <StrictMode>
      <ThemeProvider>
        <Wizard />
      </ThemeProvider>
    </StrictMode>,
  ),
);
