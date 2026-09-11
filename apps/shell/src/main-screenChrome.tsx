// Entry point for the screen's decorations.

import { ScreenChrome } from "./surfaces/screenChrome/ScreenChrome";
import { mountSurface } from "./shell/mount";

mountSurface("screenChrome", <ScreenChrome />);
