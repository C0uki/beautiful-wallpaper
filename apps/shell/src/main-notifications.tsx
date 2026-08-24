// Entry point for the toast stack.

import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { ThemeProvider } from "./design/ThemeProvider";
import { Toasts } from "./surfaces/notifications/Toasts";
import { mountSurface } from "./shell/mount";

mountSurface("notifications", (root) =>
  createRoot(root).render(
    <StrictMode>
      <ThemeProvider>
        <Toasts />
      </ThemeProvider>
    </StrictMode>,
  ),
);
