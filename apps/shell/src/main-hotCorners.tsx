// Entry point for the hot corners.

import { HotCorners } from "./surfaces/hotCorners/HotCorners";
import { mountSurface } from "./shell/mount";

mountSurface("hotCorners", <HotCorners />);
