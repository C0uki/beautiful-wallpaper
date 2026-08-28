// Entry point for the session screen.

import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { ThemeProvider } from "./design/ThemeProvider";
import { Session } from "./surfaces/session/Session";
import { mountSurface } from "./shell/mount";

mountSurface("session", (root) =>
  createRoot(root).render(
    <StrictMode>
      <ThemeProvider>
        <Session />
      </ThemeProvider>
    </StrictMode>,
  ),
);
