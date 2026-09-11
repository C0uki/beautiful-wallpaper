// Entry point for the settings screen.

import { Settings } from "./surfaces/settings/Settings";
import { mountSurface } from "./shell/mount";

mountSurface("settings", <Settings />);
