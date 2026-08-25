// Brightness, volume and microphone, in one card.
//
// The brightness slider reproduces the original's split: the bottom 30% of the
// travel is colour temperature rather than backlight, so a display already at
// its dimmest can still be taken further by warming it. Above 30% the tint is
// neutral and the slider is real brightness.

import { Slider, Symbol } from "../../widgets";
import { tr } from "../../i18n";
import { actions, useShell } from "../../shell/store";

/** Where real brightness starts; below this the slider drives the tint. */
const TINT_SHARE = 0.3;

/** The warmest the tint goes at the very bottom of the travel. */
const WARMEST_KELVIN = 2000;
const NEUTRAL_KELVIN = 6500;

/** Maps a slider position onto what it means for the display. */
export function positionToDisplay(position: number): {
  brightness: number;
  kelvin: number;
} {
  const fraction = Math.min(Math.max(position, 0), 100) / 100;
  if (fraction >= TINT_SHARE) {
    return {
      brightness: ((fraction - TINT_SHARE) / (1 - TINT_SHARE)) * 100,
      kelvin: NEUTRAL_KELVIN,
    };
  }
  return {
    brightness: 0,
    kelvin:
      WARMEST_KELVIN +
      (fraction / TINT_SHARE) * (NEUTRAL_KELVIN - WARMEST_KELVIN),
  };
}

/** The inverse, for drawing the handle where the display actually is. */
export function displayToPosition(brightness: number): number {
  return (TINT_SHARE + (brightness / 100) * (1 - TINT_SHARE)) * 100;
}

function volumeIcon(percent: number, muted: boolean): string {
  if (muted || percent <= 0) return "volume_off";
  return percent < 50 ? "volume_down" : "volume_up";
}

export function QuickSliders() {
  const config = useShell((state) => state.config.sidebar.quickSliders);
  const brightness = useShell((state) => state.brightness);
  const volume = useShell((state) => state.volume);
  const mic = useShell((state) => state.mic);

  const showBrightness = config.showBrightness && brightness.supported;
  if (!config.enable) return null;
  if (!showBrightness && !config.showVolume && !config.showMic) return null;

  return (
    <div className="bw-card bw-quick-sliders">
      {showBrightness ? (
        <div className="bw-quick-slider">
          <Symbol
            name={
              (brightness.percent ?? 0) < 40
                ? "brightness_low"
                : "brightness_high"
            }
            size={18}
          />
          <Slider
            label={tr("Brightness")}
            value={displayToPosition(brightness.percent ?? 0)}
            onChange={(position) => {
              const { brightness: level } = positionToDisplay(position);
              void actions.setBrightness(Math.round(level));
            }}
          />
        </div>
      ) : null}

      {config.showVolume ? (
        <div className="bw-quick-slider">
          <button
            type="button"
            className="bw-quick-slider-icon"
            aria-label={volume.muted ? tr("Unmute") : tr("Mute")}
            onClick={() => void actions.setMuted(!volume.muted)}
          >
            <Symbol name={volumeIcon(volume.percent, volume.muted)} size={18} />
          </button>
          <Slider
            label={tr("Volume")}
            value={volume.muted ? 0 : volume.percent}
            onChange={(percent) => void actions.setVolume(percent)}
          />
        </div>
      ) : null}

      {config.showMic ? (
        <div className="bw-quick-slider">
          <button
            type="button"
            className="bw-quick-slider-icon"
            aria-label={mic.muted ? tr("Unmute") : tr("Mute")}
            onClick={() => void actions.setMicMuted(!mic.muted)}
          >
            <Symbol name={mic.muted ? "mic_off" : "mic"} size={18} />
          </button>
          <Slider
            label={tr("Microphone")}
            value={mic.muted ? 0 : mic.percent}
            onChange={(percent) => void actions.setMic(percent)}
          />
        </div>
      ) : null}
    </div>
  );
}
