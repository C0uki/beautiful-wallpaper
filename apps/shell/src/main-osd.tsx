// Entry point for the volume and brightness readout.

import { Osd } from "./surfaces/osd/Osd";
import { mountSurface } from "./shell/mount";

mountSurface(
  "osd",
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
  </div>,
);
