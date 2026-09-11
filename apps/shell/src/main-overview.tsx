// Entry point for the search overlay.

import { Overview } from "./surfaces/overview/Overview";
import { mountSurface } from "./shell/mount";

mountSurface("overview", <Overview />);
