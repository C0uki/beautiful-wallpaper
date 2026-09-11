// Entry point for the drop shelf.

import { Shelf } from "./surfaces/shelf/Shelf";
import { mountSurface } from "./shell/mount";

mountSurface("shelf", <Shelf />);
