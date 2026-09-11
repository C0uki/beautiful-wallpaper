// Entry point for the overlay surface.

import { Overlay } from "./surfaces/overlay/Overlay";
import { mountSurface } from "./shell/mount";

mountSurface("overlay", <Overlay />);
