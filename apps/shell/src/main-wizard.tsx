// Entry point for the first-run screen.

import { Wizard } from "./surfaces/wizard/Wizard";
import { mountSurface } from "./shell/mount";

mountSurface("wizard", <Wizard />);
