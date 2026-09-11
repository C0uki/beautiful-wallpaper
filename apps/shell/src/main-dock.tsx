// Entry point for the dock.

import { Dock } from "./surfaces/dock/Dock";
import { mountSurface } from "./shell/mount";

mountSurface("dock", <Dock />);
