// Entry point for the toast stack.

import { Toasts } from "./surfaces/notifications/Toasts";
import { mountSurface } from "./shell/mount";

mountSurface("notifications", <Toasts />);
