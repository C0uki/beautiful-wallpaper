// Entry point for the right sidebar.

import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { ThemeProvider } from "./design/ThemeProvider";
import { SidebarRight } from "./surfaces/sidebarRight/SidebarRight";
import { mountSurface } from "./shell/mount";

mountSurface("sidebarRight", (root) =>
  createRoot(root).render(
    <StrictMode>
      <ThemeProvider>
        <SidebarRight />
      </ThemeProvider>
    </StrictMode>,
  ),
);
