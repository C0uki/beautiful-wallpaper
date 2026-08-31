// Entry point for the desktop menu.

import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { ThemeProvider } from "./design/ThemeProvider";
import { DesktopMenu } from "./surfaces/desktopMenu/DesktopMenu";
import { mountSurface } from "./shell/mount";

mountSurface("desktopMenu", (root) =>
  createRoot(root).render(
    <StrictMode>
      <ThemeProvider>
        <DesktopMenu />
      </ThemeProvider>
    </StrictMode>,
  ),
);
