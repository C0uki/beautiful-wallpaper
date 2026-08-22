//! Wallpaper → Material 3 colour scheme.
//!
//! This replaces end4-pC's `scripts/colors/switchwall.sh` pipeline, which shells
//! out to `matugen`, `generate_colors_material.py` (PIL + materialyoucolor) and
//! `scheme_for_image.py` (OpenCV). All three are reimplemented in-process here.
//!
//! The output deliberately keeps matugen's shape — a flat map of snake_case
//! Material role names to `#rrggbb` strings — because that is the seam the rest
//! of the shell was written against.

mod heuristic;

pub use heuristic::{colorfulness, pick_variant, Colorfulness};

use material_colors::{
    blend::harmonize,
    color::Argb,
    dynamic_color::Variant,
    image::{FilterType, ImageReader},
    scheme::Scheme,
    theme::ThemeBuilder,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;
use ts_rs::TS;

#[derive(Debug, thiserror::Error)]
pub enum ThemeError {
    #[error("could not read the wallpaper at {path}: {detail}")]
    Image { path: String, detail: String },
    #[error("`{0}` is not a Material scheme variant")]
    UnknownVariant(String),
    #[error("`{0}` is not a #rrggbb colour")]
    BadColor(String),
}

/// Light or dark, already resolved — `"auto"` is decided before we get here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum Mode {
    Light,
    Dark,
}

impl Mode {
    pub fn as_str(self) -> &'static str {
        match self {
            Mode::Light => "light",
            Mode::Dark => "dark",
        }
    }
}

/// A generated scheme, in the shape the frontend consumes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GeneratedTheme {
    pub mode: Mode,
    /// The Material variant actually used, after `"auto"` is resolved.
    pub variant: String,
    /// The colour extracted from the wallpaper, as `#rrggbb`.
    pub source: String,
    /// Material role name (snake_case) → `#rrggbb`.
    pub colors: BTreeMap<String, String>,
    /// How saturated and bright the wallpaper is, in `0.0..=1.0`. The frontend
    /// turns this into the background translucency, matching the fitted curve in
    /// end4-pC's `Appearance.qml`.
    pub wallpaper_vibrancy: f64,
}

/// Extracts the dominant colour of an image and builds a scheme from it.
pub fn from_wallpaper(
    path: &Path,
    mode: Mode,
    requested_variant: &str,
) -> Result<GeneratedTheme, ThemeError> {
    let mut image = ImageReader::open(path).map_err(|error| ThemeError::Image {
        path: path.display().to_string(),
        detail: error.to_string(),
    })?;
    // Quantising a full-resolution wallpaper is needlessly slow and no more
    // accurate; the reference implementation samples at this size too.
    image.resize(128, 128, FilterType::Lanczos3);

    let source = ImageReader::extract_color(&image);
    let variant = if requested_variant.eq_ignore_ascii_case("auto") {
        pick_variant(&heuristic::decode(path)?)
    } else {
        parse_variant(requested_variant)?
    };

    Ok(build(source, mode, variant))
}

/// Builds a scheme from an explicit accent colour, bypassing the wallpaper.
pub fn from_accent(
    accent: &str,
    mode: Mode,
    requested_variant: &str,
) -> Result<GeneratedTheme, ThemeError> {
    let source = parse_hex(accent)?;
    let variant = if requested_variant.eq_ignore_ascii_case("auto") {
        Variant::TonalSpot
    } else {
        parse_variant(requested_variant)?
    };
    Ok(build(source, mode, variant))
}

fn build(source: Argb, mode: Mode, variant: Variant) -> GeneratedTheme {
    let theme = ThemeBuilder::with_source(source)
        .variant(variant.clone())
        .build();
    let scheme = match mode {
        Mode::Light => &theme.schemes.light,
        Mode::Dark => &theme.schemes.dark,
    };

    let mut colors = roles(scheme);
    add_success_roles(&mut colors, source, mode);
    add_terminal_roles(&mut colors, source);

    GeneratedTheme {
        mode,
        variant: variant_name(variant).to_owned(),
        source: hex(source),
        wallpaper_vibrancy: vibrancy(source),
        colors,
    }
}

/// The standard Material 3 roles, under matugen's key names.
fn roles(scheme: &Scheme) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    let mut put = |key: &str, value: Argb| {
        map.insert(key.to_owned(), hex(value));
    };

    put("primary", scheme.primary);
    put("on_primary", scheme.on_primary);
    put("primary_container", scheme.primary_container);
    put("on_primary_container", scheme.on_primary_container);
    put("inverse_primary", scheme.inverse_primary);
    put("primary_fixed", scheme.primary_fixed);
    put("primary_fixed_dim", scheme.primary_fixed_dim);
    put("on_primary_fixed", scheme.on_primary_fixed);
    put("on_primary_fixed_variant", scheme.on_primary_fixed_variant);

    put("secondary", scheme.secondary);
    put("on_secondary", scheme.on_secondary);
    put("secondary_container", scheme.secondary_container);
    put("on_secondary_container", scheme.on_secondary_container);
    put("secondary_fixed", scheme.secondary_fixed);
    put("secondary_fixed_dim", scheme.secondary_fixed_dim);
    put("on_secondary_fixed", scheme.on_secondary_fixed);
    put(
        "on_secondary_fixed_variant",
        scheme.on_secondary_fixed_variant,
    );

    put("tertiary", scheme.tertiary);
    put("on_tertiary", scheme.on_tertiary);
    put("tertiary_container", scheme.tertiary_container);
    put("on_tertiary_container", scheme.on_tertiary_container);
    put("tertiary_fixed", scheme.tertiary_fixed);
    put("tertiary_fixed_dim", scheme.tertiary_fixed_dim);
    put("on_tertiary_fixed", scheme.on_tertiary_fixed);
    put(
        "on_tertiary_fixed_variant",
        scheme.on_tertiary_fixed_variant,
    );

    put("error", scheme.error);
    put("on_error", scheme.on_error);
    put("error_container", scheme.error_container);
    put("on_error_container", scheme.on_error_container);

    put("surface", scheme.surface);
    put("on_surface", scheme.on_surface);
    put("surface_dim", scheme.surface_dim);
    put("surface_bright", scheme.surface_bright);
    put("surface_tint", scheme.surface_tint);
    put("surface_variant", scheme.surface_variant);
    put("on_surface_variant", scheme.on_surface_variant);
    put("surface_container_lowest", scheme.surface_container_lowest);
    put("surface_container_low", scheme.surface_container_low);
    put("surface_container", scheme.surface_container);
    put("surface_container_high", scheme.surface_container_high);
    put(
        "surface_container_highest",
        scheme.surface_container_highest,
    );
    put("inverse_surface", scheme.inverse_surface);
    put("inverse_on_surface", scheme.inverse_on_surface);

    put("background", scheme.background);
    put("on_background", scheme.on_background);
    put("outline", scheme.outline);
    put("outline_variant", scheme.outline_variant);
    put("shadow", scheme.shadow);
    put("scrim", scheme.scrim);

    map
}

/// end4-pC extends Material with a `success` quad. Reproduce it by harmonising a
/// fixed green towards the wallpaper's hue, the same way Material harmonises its
/// own custom colours.
fn add_success_roles(map: &mut BTreeMap<String, String>, source: Argb, mode: Mode) {
    const SEED_GREEN: Argb = Argb::new(255, 76, 175, 80);
    let base = harmonize(SEED_GREEN, source);
    let theme = ThemeBuilder::with_source(base).build();
    let scheme = match mode {
        Mode::Light => &theme.schemes.light,
        Mode::Dark => &theme.schemes.dark,
    };
    map.insert("success".into(), hex(scheme.primary));
    map.insert("on_success".into(), hex(scheme.on_primary));
    map.insert("success_container".into(), hex(scheme.primary_container));
    map.insert(
        "on_success_container".into(),
        hex(scheme.on_primary_container),
    );
}

/// A 16-colour terminal palette harmonised towards the wallpaper, mirroring
/// `generate_colors_material.py`. Used for the Windows Terminal scheme export.
fn add_terminal_roles(map: &mut BTreeMap<String, String>, source: Argb) {
    // The same gruvbox-ish base the original starts from.
    const BASE: [(u8, u8, u8); 16] = [
        (40, 40, 40),
        (204, 36, 29),
        (152, 151, 26),
        (215, 153, 33),
        (69, 133, 136),
        (177, 98, 134),
        (104, 157, 106),
        (168, 153, 132),
        (146, 131, 116),
        (251, 73, 52),
        (184, 187, 38),
        (250, 189, 47),
        (131, 165, 152),
        (211, 134, 155),
        (142, 192, 124),
        (235, 219, 178),
    ];
    for (index, (r, g, b)) in BASE.into_iter().enumerate() {
        let harmonized = harmonize(Argb::new(255, r, g, b), source);
        map.insert(format!("term{index}"), hex(harmonized));
    }
}

/// `(saturation + lightness) / 2` of the source colour, in HSL terms — the input
/// to end4-pC's auto-transparency curve.
fn vibrancy(color: Argb) -> f64 {
    let r = f64::from(color.red) / 255.0;
    let g = f64::from(color.green) / 255.0;
    let b = f64::from(color.blue) / 255.0;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let lightness = (max + min) / 2.0;
    let saturation = if (max - min).abs() < f64::EPSILON {
        0.0
    } else {
        (max - min) / (1.0 - (2.0 * lightness - 1.0).abs())
    };
    ((saturation + lightness) / 2.0).clamp(0.0, 1.0)
}

fn hex(color: Argb) -> String {
    format!("#{:02x}{:02x}{:02x}", color.red, color.green, color.blue)
}

fn parse_hex(text: &str) -> Result<Argb, ThemeError> {
    let digits = text.strip_prefix('#').unwrap_or(text);
    if digits.len() != 6 {
        return Err(ThemeError::BadColor(text.to_owned()));
    }
    let channel = |from: usize| {
        u8::from_str_radix(&digits[from..from + 2], 16)
            .map_err(|_| ThemeError::BadColor(text.to_owned()))
    };
    Ok(Argb::new(255, channel(0)?, channel(2)?, channel(4)?))
}

fn parse_variant(name: &str) -> Result<Variant, ThemeError> {
    // Accept both matugen's `scheme-tonal-spot` spelling and plain camelCase.
    let normalized: String = name
        .trim_start_matches("scheme-")
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_lowercase())
        .collect();

    Ok(match normalized.as_str() {
        "monochrome" => Variant::Monochrome,
        "neutral" => Variant::Neutral,
        "tonalspot" => Variant::TonalSpot,
        "vibrant" => Variant::Vibrant,
        "expressive" => Variant::Expressive,
        "fidelity" => Variant::Fidelity,
        "content" => Variant::Content,
        "rainbow" => Variant::Rainbow,
        "fruitsalad" => Variant::FruitSalad,
        _ => return Err(ThemeError::UnknownVariant(name.to_owned())),
    })
}

fn variant_name(variant: Variant) -> &'static str {
    match variant {
        Variant::Monochrome => "monochrome",
        Variant::Neutral => "neutral",
        Variant::TonalSpot => "tonalSpot",
        Variant::Vibrant => "vibrant",
        Variant::Expressive => "expressive",
        Variant::Fidelity => "fidelity",
        Variant::Content => "content",
        Variant::Rainbow => "rainbow",
        Variant::FruitSalad => "fruitSalad",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ACCENT: &str = "#7c4dff";

    #[test]
    fn accent_scheme_covers_every_role_the_frontend_binds_to() {
        let theme = from_accent(ACCENT, Mode::Dark, "tonalSpot").unwrap();
        for role in [
            "primary",
            "on_primary",
            "primary_container",
            "surface",
            "surface_container_high",
            "on_surface_variant",
            "outline",
            "error",
            "success",
            "on_success_container",
            "term0",
            "term15",
        ] {
            let value = theme.colors.get(role).unwrap_or_else(|| panic!("{role}"));
            assert!(
                value.len() == 7 && value.starts_with('#'),
                "{role} = {value}"
            );
        }
    }

    #[test]
    fn light_and_dark_differ() {
        let light = from_accent(ACCENT, Mode::Light, "tonalSpot").unwrap();
        let dark = from_accent(ACCENT, Mode::Dark, "tonalSpot").unwrap();
        assert_ne!(light.colors["surface"], dark.colors["surface"]);
        assert_eq!(light.mode, Mode::Light);
        assert_eq!(dark.variant, "tonalSpot");
    }

    #[test]
    fn variant_names_accept_both_spellings() {
        assert_eq!(
            from_accent(ACCENT, Mode::Dark, "scheme-tonal-spot")
                .unwrap()
                .variant,
            "tonalSpot"
        );
        assert_eq!(
            from_accent(ACCENT, Mode::Dark, "fruitSalad")
                .unwrap()
                .variant,
            "fruitSalad"
        );
        assert!(matches!(
            from_accent(ACCENT, Mode::Dark, "nope"),
            Err(ThemeError::UnknownVariant(_))
        ));
    }

    #[test]
    fn variants_actually_change_the_palette() {
        let neutral = from_accent(ACCENT, Mode::Dark, "neutral").unwrap();
        let vibrant = from_accent(ACCENT, Mode::Dark, "vibrant").unwrap();
        assert_ne!(neutral.colors["primary"], vibrant.colors["primary"]);
    }

    #[test]
    fn bad_accents_are_rejected() {
        assert!(matches!(
            from_accent("#12345", Mode::Dark, "tonalSpot"),
            Err(ThemeError::BadColor(_))
        ));
        assert!(matches!(
            from_accent("not a colour", Mode::Dark, "tonalSpot"),
            Err(ThemeError::BadColor(_))
        ));
    }

    #[test]
    fn vibrancy_is_low_for_grey_and_high_for_saturated_mid_tones() {
        // Mid grey has no saturation, so vibrancy is half its lightness.
        let grey = vibrancy(Argb::new(255, 128, 128, 128));
        assert!((grey - 0.251).abs() < 0.005, "{grey}");
        assert!(vibrancy(Argb::new(255, 255, 0, 0)) > 0.7);
        assert!(vibrancy(Argb::new(255, 0, 0, 0)) < f64::EPSILON);
        assert_eq!(vibrancy(Argb::new(255, 255, 255, 255)), 0.5);
    }

    #[test]
    fn hex_round_trips() {
        assert_eq!(hex(parse_hex("#7C4DFF").unwrap()), "#7c4dff");
        assert_eq!(hex(parse_hex("7c4dff").unwrap()), "#7c4dff");
    }
}
