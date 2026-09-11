// Entry point for the desktop menu.

import { DesktopMenu } from "./surfaces/desktopMenu/DesktopMenu";
import { mountSurface } from "./shell/mount";

mountSurface("desktopMenu", <DesktopMenu />);
