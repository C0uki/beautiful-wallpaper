// Entry point for the volume and brightness readout.

import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { ThemeProvider } from "./design/ThemeProvider";
import { Osd } from "./surfaces/osd/Osd";
import { mountSurface } from "./shell/mount";

mountSurface("osd", (root) =>
  createRoot(root).render(
    <StrictMode>
      <ThemeProvider>
        <div
          style={{
            width: "100%",
            height: "100%",
            display: "grid",
            placeItems: "center",
            padding: 8,
          }}
        >
          <Osd />
        </div>
      </ThemeProvider>
    </StrictMode>,
  ),
);
