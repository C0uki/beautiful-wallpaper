// Entry point for the right sidebar.

import { SidebarRight } from "./surfaces/sidebarRight/SidebarRight";
import { mountSurface } from "./shell/mount";

mountSurface("sidebarRight", <SidebarRight />);
