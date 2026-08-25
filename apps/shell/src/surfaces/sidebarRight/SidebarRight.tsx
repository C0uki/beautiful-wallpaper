// The right sidebar: the shell's control centre.
//
// Six sections down a column — banner, quick toggles, sliders, media,
// notifications, and a tabbed group — with the dialogs drawn over the panel
// rather than in windows of their own. A separate Win32 window per dialog
// would need its own capability entry, layering and focus handling, all for
// something that only ever appears over this one panel.

import { useEffect, useState } from "react";
import { IconButton, ScrollArea } from "../../widgets";
import { tr } from "../../i18n";
import { actions, connectSidebar, useShell } from "../../shell/store";
import { Banner } from "./Banner";
import { BottomGroup } from "./BottomGroup";
import { MediaCard } from "./MediaCard";
import { NotificationCentre } from "./NotificationCentre";
import { QuickSliders } from "./QuickSliders";
import { QuickToggles } from "./QuickToggles";
import { BluetoothDialog } from "./dialogs/BluetoothDialog";
import { NightLightDialog } from "./dialogs/NightLightDialog";
import { VolumeMixer } from "./dialogs/VolumeMixer";
import { WifiDialog } from "./dialogs/WifiDialog";
import type { DetailDialog } from "./toggles";
import "../panel.css";
import "./sidebar.css";

export function SidebarRight() {
  const config = useShell((state) => state.config.sidebar);
  const style = useShell((state) => state.config.sidebar.quickToggles.style);
  const open = useShell((state) => state.states.sidebarRightOpen);
  const ready = useShell((state) => state.ready);

  const [dialog, setDialog] = useState<DetailDialog | null>(null);
  const [editing, setEditing] = useState(false);

  useEffect(() => {
    void connectSidebar();
  }, []);

  // Closing the panel closes whatever was open over it, so reopening does not
  // land the user back inside a dialog they had finished with.
  useEffect(() => {
    if (!open) {
      setDialog(null);
      setEditing(false);
    }
  }, [open]);

  // Escape closes the panel, as it does every other overlay. A dialog handles
  // its own Escape first, so this only fires once the panel is all that is up.
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key !== "Escape" || dialog) return;
      void actions.setState("sidebarRightOpen", false);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [dialog]);

  if (!ready) return null;

  return (
    <div className="bw-sidebar" data-open={open}>
      <ScrollArea className="bw-sidebar-scroll">
        <Banner />

        <div className="bw-sidebar-toggles-head">
          <QuickToggles editing={editing} onOpenDialog={setDialog} />
          {style === "android" ? (
            <IconButton
              icon={editing ? "check" : "edit"}
              size={32}
              label={tr("Edit quick toggles")}
              onClick={() => setEditing((value) => !value)}
            />
          ) : null}
        </div>

        <QuickSliders />
        {config.mediaPlayer ? <MediaCard /> : null}
        {config.notificationCentre ? <NotificationCentre /> : null}
        <BottomGroup />
      </ScrollArea>

      {dialog === "wifi" ? (
        <WifiDialog onDismiss={() => setDialog(null)} />
      ) : null}
      {dialog === "bluetooth" ? (
        <BluetoothDialog onDismiss={() => setDialog(null)} />
      ) : null}
      {dialog === "mixer" ? (
        <VolumeMixer onDismiss={() => setDialog(null)} />
      ) : null}
      {dialog === "nightLight" ? (
        <NightLightDialog onDismiss={() => setDialog(null)} />
      ) : null}
    </div>
  );
}
