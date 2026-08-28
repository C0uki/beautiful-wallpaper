// Entry point for the region picker.

import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { ThemeProvider } from "./design/ThemeProvider";
import { RegionSelect } from "./surfaces/regionSelect/RegionSelect";
import { mountSurface } from "./shell/mount";

mountSurface("regionSelect", (root) =>
  createRoot(root).render(
    <StrictMode>
      <ThemeProvider>
        <RegionSelect />
      </ThemeProvider>
    </StrictMode>,
  ),
);
