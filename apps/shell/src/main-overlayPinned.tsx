// Entry point for the overlayPinned surface.

import { OverlayPinned } from "./surfaces/overlay/OverlayPinned";
import { mountSurface } from "./shell/mount";

mountSurface("overlayPinned", <OverlayPinned />);
