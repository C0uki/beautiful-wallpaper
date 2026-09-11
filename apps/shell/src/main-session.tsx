// Entry point for the session screen.

import { Session } from "./surfaces/session/Session";
import { mountSurface } from "./shell/mount";

mountSurface("session", <Session />);
