// Entry point for the region picker.

import { RegionSelect } from "./surfaces/regionSelect/RegionSelect";
import { mountSurface } from "./shell/mount";

mountSurface("regionSelect", <RegionSelect />);
