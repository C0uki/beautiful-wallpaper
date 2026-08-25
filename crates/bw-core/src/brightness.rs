//! The arithmetic behind the brightness control and the night light.
//!
//! The Win32 half of this — WMI for laptop panels, DDC/CI for external
//! displays, `SetDeviceGammaRamp` for the tint — lives in the shell crate and
//! cannot be run anywhere but Windows. The parts that are only arithmetic live
//! here instead, because the shell crate's tests do not run on Linux at all
//! (Tauri needs gdk/webkit2gtk to build) and untested arithmetic is exactly
//! what produces a display stuck at 3% brightness.

/// Converts a raw level to a percentage.
///
/// DDC/CI monitors report an arbitrary range, not 0–100: `GetMonitorBrightness`
/// hands back a minimum, a current and a maximum, and plenty of displays use
/// something like 0–64 or 20–100. Treating the raw value as a percentage is the
/// classic way to end up with a slider that only covers half its travel.
pub fn to_percent(raw: u32, min: u32, max: u32) -> u8 {
    if max <= min {
        return 0;
    }
    let raw = raw.clamp(min, max);
    let span = f64::from(max - min);
    let position = f64::from(raw - min) / span;
    (position * 100.0).round().clamp(0.0, 100.0) as u8
}

/// The inverse: a percentage back into the display's own range.
pub fn from_percent(percent: u8, min: u32, max: u32) -> u32 {
    if max <= min {
        return min;
    }
    let span = f64::from(max - min);
    let raw = f64::from(min) + f64::from(percent.min(100)) / 100.0 * span;
    (raw.round() as u32).clamp(min, max)
}

/// The neutral colour temperature. At this value the night light is a no-op.
pub const NEUTRAL_KELVIN: u32 = 6500;

/// The range the shell will accept. Below 1000 K the approximation below stops
/// meaning anything, and above neutral the screen would turn blue.
pub const MIN_KELVIN: u32 = 1000;
pub const MAX_KELVIN: u32 = NEUTRAL_KELVIN;

/// The per-channel scale for a colour temperature, each in `0.0..=1.0`.
///
/// Tanner Helland's approximation of black-body colour, which is what every
/// f.lux-alike uses: cheap, and accurate enough that nobody can tell by eye.
pub fn kelvin_to_scale(kelvin: u32) -> (f32, f32, f32) {
    let kelvin = kelvin.clamp(MIN_KELVIN, 40_000);
    let temp = f64::from(kelvin) / 100.0;

    let red = if temp <= 66.0 {
        255.0
    } else {
        329.698_727_446 * (temp - 60.0).powf(-0.133_204_759_2)
    };

    let green = if temp <= 66.0 {
        99.470_802_586_1 * temp.ln() - 161.119_568_166_1
    } else {
        288.122_169_528_3 * (temp - 60.0).powf(-0.075_514_849_2)
    };

    let blue = if temp >= 66.0 {
        255.0
    } else if temp <= 19.0 {
        0.0
    } else {
        138.517_731_223_1 * (temp - 10.0).ln() - 305.044_792_730_7
    };

    let scale = |value: f64| (value / 255.0).clamp(0.0, 1.0) as f32;
    (scale(red), scale(green), scale(blue))
}

/// A gamma ramp in the layout `SetDeviceGammaRamp` expects: red, then green,
/// then blue, 256 entries each.
///
/// `dim` scales every channel on top of the tint, which is how a display with
/// no brightness control of its own still gets a usable slider — it is not as
/// good as real backlight control (the panel is still emitting the same light)
/// so the caller only reaches for it when the real thing is unavailable.
pub fn gamma_ramp(kelvin: u32, dim: f32) -> [u16; 256 * 3] {
    let (red, green, blue) = kelvin_to_scale(kelvin);
    let dim = dim.clamp(0.05, 1.0);

    let mut ramp = [0u16; 256 * 3];
    for (index, slot) in ramp.iter_mut().enumerate() {
        let channel = index / 256;
        let level = (index % 256) as f32;
        let scale = match channel {
            0 => red,
            1 => green,
            _ => blue,
        };
        // `* 257` maps 0..=255 onto 0..=65535 exactly.
        *slot = (level * 257.0 * scale * dim).round().clamp(0.0, 65_535.0) as u16;
    }
    ramp
}

/// Whether a tint would visibly change anything.
///
/// Writing a ramp is not free — it is a round trip to the driver, and some
/// drivers flash — so the caller skips the write when this is false.
pub fn is_neutral(kelvin: u32, dim: f32) -> bool {
    kelvin >= NEUTRAL_KELVIN && dim >= 1.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_zero_to_hundred_display_maps_one_to_one() {
        assert_eq!(to_percent(0, 0, 100), 0);
        assert_eq!(to_percent(50, 0, 100), 50);
        assert_eq!(to_percent(100, 0, 100), 100);
    }

    #[test]
    fn an_odd_range_still_covers_the_whole_slider() {
        // A real monitor reporting 0-64: the classic case where treating the
        // raw value as a percentage leaves the slider stuck under two thirds.
        assert_eq!(to_percent(64, 0, 64), 100);
        assert_eq!(to_percent(32, 0, 64), 50);

        // And one whose minimum is not zero.
        assert_eq!(to_percent(20, 20, 100), 0);
        assert_eq!(to_percent(60, 20, 100), 50);
        assert_eq!(to_percent(100, 20, 100), 100);
    }

    #[test]
    fn percentages_round_trip_through_the_display_range() {
        for (min, max) in [(0, 100), (0, 64), (20, 100), (10, 255)] {
            for percent in [0u8, 25, 50, 75, 100] {
                let raw = from_percent(percent, min, max);
                let back = to_percent(raw, min, max);
                assert!(
                    back.abs_diff(percent) <= 1,
                    "{percent}% became {back}% through {min}..{max}"
                );
            }
        }
    }

    #[test]
    fn a_display_reporting_nonsense_does_not_divide_by_zero() {
        // Some displays answer DDC/CI with min == max, or with them inverted.
        assert_eq!(to_percent(50, 100, 100), 0);
        assert_eq!(to_percent(50, 100, 10), 0);
        assert_eq!(from_percent(50, 100, 100), 100);
        assert_eq!(from_percent(50, 100, 10), 100);
    }

    #[test]
    fn raw_values_outside_the_range_are_clamped_not_wrapped() {
        assert_eq!(to_percent(200, 0, 100), 100);
        assert_eq!(to_percent(0, 20, 100), 0);
        assert_eq!(from_percent(200, 0, 100), 100);
    }

    #[test]
    fn neutral_light_is_white() {
        let (red, green, blue) = kelvin_to_scale(NEUTRAL_KELVIN);
        assert!(red > 0.99, "{red}");
        assert!(green > 0.95, "{green}");
        assert!(blue > 0.95, "{blue}");
    }

    #[test]
    fn warmer_means_less_blue_and_never_more() {
        let mut previous = 1.0;
        for kelvin in [6500, 5000, 4000, 3000, 2000] {
            let (red, _, blue) = kelvin_to_scale(kelvin);
            assert!(blue <= previous, "{kelvin}K raised blue to {blue}");
            // Red is what is left; a warm tint must not darken it.
            assert!(red > 0.99, "{kelvin}K dropped red to {red}");
            previous = blue;
        }
    }

    #[test]
    fn every_channel_stays_in_range() {
        for kelvin in [MIN_KELVIN, 2500, 4000, NEUTRAL_KELVIN, 40_000] {
            let (red, green, blue) = kelvin_to_scale(kelvin);
            for value in [red, green, blue] {
                assert!((0.0..=1.0).contains(&value), "{kelvin}K gave {value}");
            }
        }
    }

    #[test]
    fn a_neutral_ramp_is_the_identity_windows_expects() {
        let ramp = gamma_ramp(NEUTRAL_KELVIN, 1.0);
        // Red is unattenuated at 6500K, so its ramp is the plain i*257 curve —
        // which is what a driver reads as "no change".
        assert_eq!(ramp[0], 0);
        assert_eq!(ramp[255], 65_535);
        assert_eq!(ramp[128], 128 * 257);
    }

    #[test]
    fn a_ramp_is_monotonic_in_every_channel() {
        // A non-monotonic ramp is rejected by some drivers and produces
        // posterised colour on the rest.
        let ramp = gamma_ramp(3000, 0.7);
        for channel in 0..3 {
            let slice = &ramp[channel * 256..(channel + 1) * 256];
            for pair in slice.windows(2) {
                assert!(pair[1] >= pair[0], "channel {channel} went backwards");
            }
        }
    }

    #[test]
    fn dimming_never_blanks_the_screen_completely() {
        // A ramp of all zeroes is a black display with no way back for a user
        // who cannot see the slider they just dragged.
        let ramp = gamma_ramp(NEUTRAL_KELVIN, 0.0);
        assert!(ramp[255] > 0, "full white went to {}", ramp[255]);
    }

    #[test]
    fn only_a_real_change_is_worth_writing() {
        assert!(is_neutral(NEUTRAL_KELVIN, 1.0));
        assert!(!is_neutral(4000, 1.0));
        assert!(!is_neutral(NEUTRAL_KELVIN, 0.5));
    }
}
