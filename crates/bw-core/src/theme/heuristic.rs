//! Picking a Material scheme variant to suit an image.
//!
//! A port of end4-pC's `scripts/colors/scheme_for_image.py`, which uses OpenCV to
//! compute the Hasler–Süsstrunk colourfulness metric and then chooses a muted
//! scheme for washed-out wallpapers and a tonal one for colourful ones. The
//! metric is simple enough that `image` alone covers it, so the OpenCV and Python
//! dependencies go away.

use super::ThemeError;
use image::imageops::FilterType;
use material_colors::dynamic_color::Variant;
use std::path::Path;

/// The Hasler–Süsstrunk decision threshold used by the original script.
const COLORFUL_THRESHOLD: f64 = 40.0;

/// Colourfulness of an image, on Hasler–Süsstrunk's scale (roughly 0..150).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Colorfulness(pub f64);

impl Colorfulness {
    pub fn is_colorful(self) -> bool {
        self.0 >= COLORFUL_THRESHOLD
    }
}

/// Loads an image and measures it. Downsamples first: the metric is a summary
/// statistic, so full resolution buys nothing but time.
pub(super) fn decode(path: &Path) -> Result<Colorfulness, ThemeError> {
    let image = image::open(path).map_err(|error| ThemeError::Image {
        path: path.display().to_string(),
        detail: error.to_string(),
    })?;
    let small = image.resize_exact(256, 256, FilterType::Triangle).to_rgb8();
    Ok(colorfulness(small.as_raw()))
}

/// Hasler–Süsstrunk colourfulness over packed RGB bytes.
///
/// `rg = R - G`, `yb = ½(R + G) - B`, then
/// `√(σ²rg + σ²yb) + 0.3·√(μ²rg + μ²yb)`.
pub fn colorfulness(rgb: &[u8]) -> Colorfulness {
    let pixels = rgb.len() / 3;
    if pixels == 0 {
        return Colorfulness(0.0);
    }

    let (mut sum_rg, mut sum_yb, mut sum_rg2, mut sum_yb2) = (0.0f64, 0.0f64, 0.0f64, 0.0f64);
    // Fixed-size chunks, so each one is a `&[u8; 3]` and the channel reads below
    // are checked at compile time rather than per pixel. A trailing partial
    // pixel, if the caller passed one, is discarded either way.
    let (pixels_rgb, _) = rgb.as_chunks::<3>();
    for &[red, green, blue] in pixels_rgb {
        let (r, g, b) = (f64::from(red), f64::from(green), f64::from(blue));
        let rg = r - g;
        let yb = 0.5 * (r + g) - b;
        sum_rg += rg;
        sum_yb += yb;
        sum_rg2 += rg * rg;
        sum_yb2 += yb * yb;
    }

    let n = pixels as f64;
    let mean_rg = sum_rg / n;
    let mean_yb = sum_yb / n;
    let var_rg = (sum_rg2 / n - mean_rg * mean_rg).max(0.0);
    let var_yb = (sum_yb2 / n - mean_yb * mean_yb).max(0.0);

    let std_root = (var_rg + var_yb).sqrt();
    let mean_root = (mean_rg * mean_rg + mean_yb * mean_yb).sqrt();
    Colorfulness(std_root + 0.3 * mean_root)
}

/// Muted wallpapers get a neutral scheme so the UI does not invent colour that
/// is not in the image; colourful ones get the default tonal spot.
pub fn pick_variant(measure: &Colorfulness) -> Variant {
    if measure.is_colorful() {
        Variant::TonalSpot
    } else {
        Variant::Neutral
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid(r: u8, g: u8, b: u8, pixels: usize) -> Vec<u8> {
        std::iter::repeat_n([r, g, b], pixels).flatten().collect()
    }

    #[test]
    fn a_flat_grey_image_is_not_colorful() {
        let measure = colorfulness(&solid(128, 128, 128, 4096));
        assert_eq!(measure.0, 0.0);
        assert!(!measure.is_colorful());
        assert!(matches!(pick_variant(&measure), Variant::Neutral));
    }

    #[test]
    fn a_saturated_image_is_colorful() {
        let mut pixels = solid(255, 0, 0, 2048);
        pixels.extend(solid(0, 0, 255, 2048));
        let measure = colorfulness(&pixels);
        assert!(measure.is_colorful(), "{measure:?}");
        assert!(matches!(pick_variant(&measure), Variant::TonalSpot));
    }

    #[test]
    fn a_desaturated_photo_stays_below_the_threshold() {
        // Slight sepia tint: enough to be non-grey, not enough to be "colourful".
        let mut pixels = solid(140, 132, 120, 2048);
        pixels.extend(solid(120, 114, 104, 2048));
        assert!(!colorfulness(&pixels).is_colorful());
    }

    #[test]
    fn an_empty_buffer_does_not_divide_by_zero() {
        assert_eq!(colorfulness(&[]).0, 0.0);
    }
}
