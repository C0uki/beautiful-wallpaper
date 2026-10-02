// Bluetooth devices: the paired ones, and the ones in range to pair with.
//
// Pairing that needs a PIN asks here — the shell shows it, asks whether the
// device shows the same one, or takes one typed in — rather than sending
// the person to Settings for the one step that wants them. Connecting is
// offered for audio devices only: anything else connects itself when it is
// switched on, and Windows has no way to ask it to.

import { useCallback, useEffect, useState } from "react";
import {
  Button,
  Dialog,
  IconButton,
  ListRow,
  Placeholder,
  ScrollArea,
} from "../../../widgets";
import { tr } from "../../../i18n";
import { backend } from "../../../shell/backend";
import { actions, useShell } from "../../../shell/store";
import { Event, type BluetoothDeviceInfo, type PairingPrompt } from "@bw/core";

export function BluetoothDialog({ onDismiss }: { onDismiss: () => void }) {
  const enabled = useShell((state) => state.radios.bluetooth);
  const [devices, setDevices] = useState<BluetoothDeviceInfo[]>([]);
  // `null` until the person asks to look: looking costs the radio seconds.
  const [nearby, setNearby] = useState<BluetoothDeviceInfo[] | null>(null);
  const [searching, setSearching] = useState(false);
  const [busy, setBusy] = useState<string | null>(null);
  const [prompt, setPrompt] = useState<PairingPrompt | null>(null);
  const [typed, setTyped] = useState("");

  const refresh = useCallback(() => {
    void actions.bluetoothDevices().then(setDevices);
  }, []);

  useEffect(() => {
    if (!enabled) {
      setDevices([]);
      setNearby(null);
      return;
    }
    refresh();
  }, [enabled, refresh]);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let gone = false;
    void backend()
      .listen<PairingPrompt>(Event.BluetoothPairing, (asked) => {
        setTyped("");
        setPrompt(asked);
      })
      .then((stop) => {
        if (gone) stop();
        else unlisten = stop;
      });
    return () => {
      gone = true;
      unlisten?.();
    };
  }, []);

  /** Runs one device's job, then reads the list again: what took, not what
   *  was asked for. */
  const work = async (id: string, job: () => Promise<unknown>) => {
    setBusy(id);
    try {
      await job();
    } finally {
      setBusy(null);
      setPrompt(null);
      refresh();
    }
  };

  const search = async () => {
    setSearching(true);
    try {
      setNearby(await actions.scanBluetooth());
    } finally {
      setSearching(false);
    }
  };

  const answer = (value: string | null) => {
    void actions.answerBluetoothPairing(value);
    setPrompt(null);
  };

  return (
    <Dialog
      title={tr("Bluetooth")}
      icon="bluetooth"
      onDismiss={onDismiss}
      footer={
        <>
          {enabled ? (
            <Button
              variant="text"
              disabled={searching}
              onClick={() => void search()}
            >
              {searching ? tr("Searching…") : tr("Search for devices")}
            </Button>
          ) : null}
          <Button
            variant="tonal"
            onClick={() =>
              void backend().invoke("plugin:opener|open_url", {
                url: "ms-settings:bluetooth",
              })
            }
          >
            {tr("Windows settings")}
          </Button>
        </>
      }
    >
      {prompt ? (
        <Prompt
          prompt={prompt}
          typed={typed}
          setTyped={setTyped}
          answer={answer}
        />
      ) : null}

      {!enabled ? (
        <Placeholder icon="bluetooth_disabled" text={tr("Bluetooth is off")} />
      ) : (
        <ScrollArea>
          {devices.length === 0 ? (
            <Placeholder
              icon="bluetooth_searching"
              text={tr("No paired devices")}
            />
          ) : (
            devices.map((device) => (
              <ListRow
                key={device.id}
                icon={device.connected ? "bluetooth_connected" : "bluetooth"}
                title={device.name}
                detail={
                  busy === device.id
                    ? tr("Connecting…")
                    : device.connected
                      ? tr("Connected")
                      : tr("Not connected")
                }
                trailing={
                  <>
                    {device.audio ? (
                      <IconButton
                        icon={device.connected ? "link_off" : "link"}
                        size={32}
                        label={
                          device.connected ? tr("Disconnect") : tr("Connect")
                        }
                        disabled={busy !== null}
                        onClick={() =>
                          void work(device.id, () =>
                            actions.connectBluetooth(
                              device.id,
                              !device.connected,
                            ),
                          )
                        }
                      />
                    ) : null}
                    <IconButton
                      icon="delete"
                      size={32}
                      label={tr("Forget")}
                      disabled={busy !== null}
                      onClick={() =>
                        void work(device.id, () =>
                          actions.unpairBluetooth(device.id),
                        )
                      }
                    />
                  </>
                }
              />
            ))
          )}

          {nearby ? (
            <>
              <h3 className="bw-dialog-subheading">{tr("Nearby devices")}</h3>
              {nearby.length === 0 ? (
                <Placeholder
                  icon="bluetooth_searching"
                  text={tr("No devices found")}
                />
              ) : (
                nearby.map((device) => (
                  <ListRow
                    key={device.id}
                    icon="bluetooth"
                    title={device.name}
                    {...(busy === device.id ? { detail: tr("Pairing…") } : {})}
                    trailing={
                      <Button
                        variant="tonal"
                        disabled={busy !== null}
                        onClick={() =>
                          void work(device.id, async () => {
                            if (await actions.pairBluetooth(device.id)) {
                              setNearby((list) =>
                                (list ?? []).filter(
                                  (each) => each.id !== device.id,
                                ),
                              );
                            }
                          })
                        }
                      >
                        {tr("Pair")}
                      </Button>
                    }
                  />
                ))
              )}
            </>
          ) : null}
        </ScrollArea>
      )}
    </Dialog>
  );
}

/** What pairing needs from the person, in the words the PIN calls for. */
function Prompt({
  prompt,
  typed,
  setTyped,
  answer,
}: {
  prompt: PairingPrompt;
  typed: string;
  setTyped: (value: string) => void;
  answer: (value: string | null) => void;
}) {
  if (prompt.kind === "displayPin") {
    return (
      <p className="bw-bluetooth-prompt">
        {tr("Type %1 on the device, then press Enter.").replace(
          "%1",
          prompt.pin,
        )}
      </p>
    );
  }
  if (prompt.kind === "confirmPin") {
    return (
      <div className="bw-bluetooth-prompt">
        <p>{tr("Does the device show %1?").replace("%1", prompt.pin)}</p>
        <Button variant="text" onClick={() => answer(null)}>
          {tr("No")}
        </Button>
        <Button variant="tonal" onClick={() => answer("yes")}>
          {tr("Yes")}
        </Button>
      </div>
    );
  }
  return (
    <form
      className="bw-bluetooth-prompt"
      onSubmit={(event) => {
        event.preventDefault();
        answer(typed);
      }}
    >
      <p>{tr("Type the device's PIN.")}</p>
      <input
        autoFocus
        value={typed}
        inputMode="numeric"
        onChange={(event) => setTyped(event.target.value)}
      />
      <Button variant="text" type="button" onClick={() => answer(null)}>
        {tr("Cancel")}
      </Button>
      <Button variant="tonal" type="submit">
        {tr("Pair")}
      </Button>
    </form>
  );
}
