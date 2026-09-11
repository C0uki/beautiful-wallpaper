// Entry point for the background surface (wallpaper + desktop widgets).

import { Background } from "./surfaces/background/Background";
import { mountSurface } from "./shell/mount";

mountSurface("background", <Background />);
