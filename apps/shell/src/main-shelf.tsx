// Entry point for the drop shelf.

import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { ThemeProvider } from "./design/ThemeProvider";
import { Shelf } from "./surfaces/shelf/Shelf";
import { mountSurface } from "./shell/mount";

mountSurface("shelf", (root) =>
  createRoot(root).render(
    <StrictMode>
      <ThemeProvider>
        <Shelf />
      </ThemeProvider>
    </StrictMode>,
  ),
);
