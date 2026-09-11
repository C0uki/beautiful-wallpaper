// Entry point for the bar surface.

import { Bar } from "./surfaces/bar/Bar";
import { mountSurface } from "./shell/mount";

mountSurface("bar", <Bar />);
