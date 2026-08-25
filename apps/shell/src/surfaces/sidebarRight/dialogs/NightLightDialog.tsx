// Night light settings.
//
// The shell applies its own gamma ramp rather than driving Windows' Night
// Light, whose settings live in an undocumented registry blob — so this is the
// only place the temperature can be set, and it says what it is doing.

import { Dialog, Slider, Switch } from "../../../widgets";
import { tr } from "../../../i18n";
import { actions, useShell } from "../../../shell/store";

const WARMEST = 2000;
const NEUTRAL = 6500;

export function NightLightDialog({ onDismiss }: { onDismiss: () => void }) {
  const nightLight = useShell((state) => state.config.sidebar.nightLight);

  return (
    <Dialog title={tr("Night light")} icon="bedtime" onDismiss={onDismiss}>
      <div className="bw-night-light">
        <label className="bw-night-light-row">
          <span>{tr("Night light")}</span>
          <Switch
            checked={nightLight.enable}
            label={tr("Night light")}
            onChange={(enable) => void actions.setNightLight(enable)}
          />
        </label>

        <div className="bw-night-light-temperature">
          <div className="bw-night-light-row">
            <span>{tr("Colour temperature")}</span>
            <span>{nightLight.temperature}K</span>
          </div>
          <Slider
            label={tr("Colour temperature")}
            min={WARMEST}
            max={NEUTRAL}
            step={100}
            value={nightLight.temperature}
            onChange={(temperature) => {
              void actions
                .setConfigValue("sidebar.nightLight.temperature", temperature)
                // Re-apply so the change is visible while the slider moves,
                // rather than only after the next toggle.
                .then(() => {
                  if (nightLight.enable) void actions.setNightLight(true);
                });
            }}
          />
        </div>

        <p className="bw-night-light-note">
          {tr(
            "Applied through the display's gamma ramp. Some displays — HDR in particular — refuse it.",
          )}
        </p>
      </div>
    </Dialog>
  );
}
