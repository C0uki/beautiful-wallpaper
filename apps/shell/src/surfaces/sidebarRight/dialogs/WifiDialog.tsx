// The Wi-Fi network picker.
//
// A scan takes seconds, so it runs when the dialog opens rather than on a
// timer, and the list shows what the last scan found while the next one runs.

import { useCallback, useEffect, useState } from "react";
import {
  Button,
  Dialog,
  ListRow,
  Placeholder,
  ScrollArea,
  Symbol,
} from "../../../widgets";
import { tr } from "../../../i18n";
import { actions, useShell } from "../../../shell/store";
import type { WifiNetwork } from "@bw/core";

/**
 * Signal strength, as one of the three wifi glyphs that survive subsetting.
 *
 * Material Symbols does have a per-bar set, but every one of those names
 * contains `_digit_` — they are aliases whose output glyph the subsetter
 * prunes, so they render as the literal word. Three states carry the meaning;
 * the exact bar count goes in the row's text, where it cannot be misread.
 */
export function signalIcon(bars: number): string {
  if (bars >= 3) return "wifi";
  if (bars >= 1) return "signal_wifi_bad";
  return "signal_wifi_statusbar_null";
}

export function WifiDialog({ onDismiss }: { onDismiss: () => void }) {
  const enabled = useShell((state) => state.radios.wifi);
  const [networks, setNetworks] = useState<WifiNetwork[]>([]);
  const [scanning, setScanning] = useState(false);
  const [selected, setSelected] = useState<WifiNetwork | null>(null);
  const [password, setPassword] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [connecting, setConnecting] = useState(false);

  const scan = useCallback(async () => {
    setScanning(true);
    try {
      setNetworks(await actions.scanWifi());
    } finally {
      setScanning(false);
    }
  }, []);

  useEffect(() => {
    if (enabled) void scan();
  }, [enabled, scan]);

  const connect = async () => {
    if (!selected) return;
    setConnecting(true);
    setError(null);
    try {
      const outcome = await actions.connectWifi(
        selected.ssid,
        selected.secured ? password : undefined,
      );
      if (outcome === "connected") {
        setSelected(null);
        setPassword("");
        return;
      }
      setError(
        outcome === "badPassword"
          ? tr("Wrong password")
          : tr("Could not connect"),
      );
    } finally {
      setConnecting(false);
    }
  };

  return (
    <Dialog
      title={tr("Wi-Fi")}
      icon="wifi"
      onDismiss={onDismiss}
      footer={
        <>
          <Button variant="text" onClick={() => void actions.disconnectWifi()}>
            {tr("Disconnect")}
          </Button>
          <Button
            variant="tonal"
            disabled={scanning || !enabled}
            onClick={() => void scan()}
          >
            {scanning ? tr("Scanning…") : tr("Scan")}
          </Button>
        </>
      }
    >
      {!enabled ? (
        <Placeholder icon="wifi_off" text={tr("Wi-Fi is off")} />
      ) : networks.length === 0 ? (
        <Placeholder
          icon="wifi_find"
          text={scanning ? tr("Scanning…") : tr("No networks found")}
        />
      ) : (
        <ScrollArea className="bw-wifi-list">
          {networks.map((network) => (
            <div key={network.ssid}>
              <ListRow
                icon={signalIcon(network.bars)}
                title={network.ssid}
                detail={`${network.secured ? tr("Secured") : tr("Open")} · ${tr(
                  "%1/4 bars",
                ).replace("%1", String(network.bars))}`}
                selected={selected?.ssid === network.ssid}
                trailing={
                  network.secured ? <Symbol name="lock" size={16} /> : undefined
                }
                onClick={() => {
                  setSelected(network);
                  setPassword("");
                  setError(null);
                }}
              />

              {selected?.ssid === network.ssid ? (
                <div className="bw-wifi-connect">
                  {network.secured ? (
                    <input
                      type="password"
                      value={password}
                      autoFocus
                      placeholder={tr("Password")}
                      aria-label={tr("Password")}
                      onChange={(event) => setPassword(event.target.value)}
                      onKeyDown={(event) => {
                        if (event.key === "Enter") void connect();
                      }}
                    />
                  ) : null}
                  {error ? (
                    <span className="bw-wifi-error">{error}</span>
                  ) : null}
                  <Button
                    variant="filled"
                    disabled={connecting}
                    onClick={() => void connect()}
                  >
                    {connecting ? tr("Connecting…") : tr("Connect")}
                  </Button>
                </div>
              ) : null}
            </div>
          ))}
        </ScrollArea>
      )}
    </Dialog>
  );
}
