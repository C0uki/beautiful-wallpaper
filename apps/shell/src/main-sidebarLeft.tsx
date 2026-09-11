// Entry point for the left sidebar.

import { SidebarLeft } from "./surfaces/sidebarLeft/SidebarLeft";
import { mountSurface } from "./shell/mount";

mountSurface("sidebarLeft", <SidebarLeft />);
