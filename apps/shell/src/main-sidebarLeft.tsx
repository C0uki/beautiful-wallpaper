// Entry point for the left sidebar.

import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { ThemeProvider } from "./design/ThemeProvider";
import { SidebarLeft } from "./surfaces/sidebarLeft/SidebarLeft";
import { mountSurface } from "./shell/mount";

mountSurface("sidebarLeft", (root) =>
  createRoot(root).render(
    <StrictMode>
      <ThemeProvider>
        <SidebarLeft />
      </ThemeProvider>
    </StrictMode>,
  ),
);
