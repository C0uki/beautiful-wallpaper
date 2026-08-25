// Paired Bluetooth devices.
//
// Pairing a new device needs a PIN exchange with a UI of its own, and Windows
// already has one — so this lists what is already paired and hands the rest
// over rather than reimplementing it badly.

import { useEffect, useState } from "react";
import {
  Button,
  Dialog,
  ListRow,
  Placeholder,
  ScrollArea,
} from "../../../widgets";
import { tr } from "../../../i18n";
import { backend } from "../../../shell/backend";
import { actions, useShell } from "../../../shell/store";
import type { BluetoothDeviceInfo } from "@bw/core";

export function BluetoothDialog({ onDismiss }: { onDismiss: () => void }) {
  const enabled = useShell((state) => state.radios.bluetooth);
  const [devices, setDevices] = useState<BluetoothDeviceInfo[]>([]);

  useEffect(() => {
    if (!enabled) {
      setDevices([]);
      return;
    }
    void actions.bluetoothDevices().then(setDevices);
  }, [enabled]);

  return (
    <Dialog
      title={tr("Bluetooth")}
      icon="bluetooth"
      onDismiss={onDismiss}
      footer={
        <Button
          variant="tonal"
          onClick={() =>
            // Connecting and pairing are largely the stack's decision rather
            // than ours, so the honest thing is to open the real settings.
            void backend().invoke("plugin:opener|open_url", {
              url: "ms-settings:bluetooth",
            })
          }
        >
          {tr("Windows settings")}
        </Button>
      }
    >
      {!enabled ? (
        <Placeholder icon="bluetooth_disabled" text={tr("Bluetooth is off")} />
      ) : devices.length === 0 ? (
        <Placeholder
          icon="bluetooth_searching"
          text={tr("No paired devices")}
        />
      ) : (
        <ScrollArea>
          {devices.map((device) => (
            <ListRow
              key={device.id}
              icon={device.connected ? "bluetooth_connected" : "bluetooth"}
              title={device.name}
              detail={device.connected ? tr("Connected") : tr("Not connected")}
            />
          ))}
        </ScrollArea>
      )}
    </Dialog>
  );
}
