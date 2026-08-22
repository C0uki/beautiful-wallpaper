//! Generating and publishing the Material 3 scheme.
//!
//! The pipeline `switchwall.sh` runs as five processes — matugen, two Python
//! scripts, `jq` and `gsettings` — happens here in one function call.

use std::path::Path;

use bw_core::{theme, GeneratedTheme, Mode};

use crate::state::AppState;

/// Regenerates the scheme for the current wallpaper and config.
///
/// Falls back to the configured accent colour when there is no wallpaper yet, or
/// when the image cannot be read — a broken path should restyle the shell, not
/// leave it unthemed.
pub fn regenerate(state: &AppState) -> Result<GeneratedTheme, String> {
    let config = state.config();
    let palette = &config.appearance.palette;

    let mode = resolve_mode(&palette.mode);
    let variant = palette.r#type.as_str();

    let generated = match palette.accent_color.as_deref() {
        Some(accent) if !accent.is_empty() => theme::from_accent(accent, mode, variant),
        _ => {
            let path = Path::new(&config.background.wallpaper_path);
            if config.background.wallpaper_path.is_empty() {
                theme::from_accent(DEFAULT_ACCENT, mode, variant)
            } else {
                theme::from_wallpaper(path, mode, variant).or_else(|error| {
                    tracing::warn!(%error, "falling back to the default accent");
                    theme::from_accent(DEFAULT_ACCENT, mode, variant)
                })
            }
        }
    }
    .map_err(|error| error.to_string())?;

    persist(&generated);

    if config.appearance.wallpaper_theming.sync_system_accent {
        sync_system(&generated);
    }
    if config.appearance.wallpaper_theming.sync_windows_terminal {
        if let Err(error) = write_windows_terminal_scheme(&generated) {
            tracing::warn!(%error, "could not update the Windows Terminal scheme");
        }
    }

    state.set_theme(generated.clone());
    Ok(generated)
}

/// The accent used before a wallpaper has been chosen.
const DEFAULT_ACCENT: &str = "#a8577a";

fn resolve_mode(configured: &str) -> Mode {
    match configured {
        "light" => Mode::Light,
        "dark" => Mode::Dark,
        // "auto" follows whatever Windows is set to, so the shell matches the
        // rest of the desktop out of the box.
        _ => {
            if system_prefers_light() {
                Mode::Light
            } else {
                Mode::Dark
            }
        }
    }
}

#[cfg(windows)]
fn system_prefers_light() -> bool {
    windows_registry::CURRENT_USER
        .open(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize")
        .and_then(|key| key.get_u32("AppsUseLightTheme"))
        .map(|value| value != 0)
        .unwrap_or(false)
}

#[cfg(not(windows))]
fn system_prefers_light() -> bool {
    false
}

/// Writes the scheme to disk in matugen's `colors.json` shape.
///
/// Nothing in the shell reads it back — the theme is passed in memory — but
/// keeping the file means external tooling written against the original still
/// has something to watch.
fn persist(theme: &GeneratedTheme) {
    let path = bw_core::paths::generated_theme_file();
    if let Some(parent) = path.parent() {
        if let Err(error) = std::fs::create_dir_all(parent) {
            tracing::warn!(%error, "could not create the generated theme directory");
            return;
        }
    }
    match serde_json::to_string_pretty(&theme.colors) {
        Ok(json) => {
            if let Err(error) = std::fs::write(&path, json) {
                tracing::warn!(%error, "could not write the generated theme");
            }
        }
        Err(error) => tracing::warn!(%error, "could not serialise the generated theme"),
    }
}

#[cfg(windows)]
fn sync_system(theme: &GeneratedTheme) {
    let Some(accent) = theme.colors.get("primary").and_then(|hex| parse_hex(hex)) else {
        return;
    };
    if let Err(error) =
        crate::platform::wallpaper::sync_system_theme(accent, theme.mode == Mode::Dark)
    {
        tracing::warn!(%error, "could not sync the system accent colour");
    }
}

#[cfg(not(windows))]
fn sync_system(_theme: &GeneratedTheme) {}

fn parse_hex(hex: &str) -> Option<(u8, u8, u8)> {
    let digits = hex.strip_prefix('#')?;
    if digits.len() != 6 {
        return None;
    }
    let channel = |at: usize| u8::from_str_radix(&digits[at..at + 2], 16).ok();
    Some((channel(0)?, channel(2)?, channel(4)?))
}

/// Adds a `beautiful-wallpaper` colour scheme to Windows Terminal.
///
/// The counterpart of the original's kitty theme generation. Only the scheme is
/// added — which profile uses it stays the user's choice, since silently
/// repointing their profiles would be an unpleasant surprise.
fn write_windows_terminal_scheme(theme: &GeneratedTheme) -> Result<(), String> {
    let Some(settings) = windows_terminal_settings() else {
        return Ok(());
    };

    let text = std::fs::read_to_string(&settings)
        .map_err(|error| format!("could not read {}: {error}", settings.display()))?;
    let mut root: serde_json::Value = serde_json::from_str(&text)
        .map_err(|error| format!("{} is not valid JSON: {error}", settings.display()))?;

    let colour = |name: &str, fallback: &str| {
        theme
            .colors
            .get(name)
            .cloned()
            .unwrap_or_else(|| fallback.to_owned())
    };
    let term = |index: usize, fallback: &str| colour(&format!("term{index}"), fallback);

    let scheme = serde_json::json!({
        "name": "beautiful-wallpaper",
        "background": colour("surface", "#141218"),
        "foreground": colour("on_surface", "#e6e0e9"),
        "cursorColor": colour("primary", "#d0bcff"),
        "selectionBackground": colour("primary_container", "#4f378b"),
        "black": term(0, "#282828"),
        "red": term(1, "#cc241d"),
        "green": term(2, "#98971a"),
        "yellow": term(3, "#d79921"),
        "blue": term(4, "#458588"),
        "purple": term(5, "#b16286"),
        "cyan": term(6, "#689d6a"),
        "white": term(7, "#a89984"),
        "brightBlack": term(8, "#928374"),
        "brightRed": term(9, "#fb4934"),
        "brightGreen": term(10, "#b8bb26"),
        "brightYellow": term(11, "#fabd2f"),
        "brightBlue": term(12, "#83a598"),
        "brightPurple": term(13, "#d3869b"),
        "brightCyan": term(14, "#8ec07c"),
        "brightWhite": term(15, "#ebdbb2"),
    });

    let schemes = root
        .get_mut("schemes")
        .and_then(serde_json::Value::as_array_mut)
        .ok_or_else(|| "Windows Terminal settings have no `schemes` array".to_owned())?;

    // Replace ours if it is already there, rather than accumulating duplicates.
    match schemes
        .iter()
        .position(|entry| entry.get("name") == Some(&serde_json::json!("beautiful-wallpaper")))
    {
        Some(index) => schemes[index] = scheme,
        None => schemes.push(scheme),
    }

    let json = serde_json::to_string_pretty(&root).map_err(|error| error.to_string())?;
    std::fs::write(&settings, json)
        .map_err(|error| format!("could not write {}: {error}", settings.display()))
}

fn windows_terminal_settings() -> Option<std::path::PathBuf> {
    let local = dirs::data_local_dir()?;
    for package in [
        "Microsoft.WindowsTerminal_8wekyb3d8bbwe",
        "Microsoft.WindowsTerminalPreview_8wekyb3d8bbwe",
    ] {
        let candidate = local
            .join("Packages")
            .join(package)
            .join("LocalState")
            .join("settings.json");
        if candidate.exists() {
            return Some(candidate);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_parses_to_channels() {
        assert_eq!(parse_hex("#ff8000"), Some((255, 128, 0)));
        assert_eq!(parse_hex("ff8000"), Some((255, 128, 0)));
        assert_eq!(parse_hex("#fff"), None);
        assert_eq!(parse_hex("nope"), None);
    }

    #[test]
    fn explicit_modes_win_over_the_system_preference() {
        assert_eq!(resolve_mode("light"), Mode::Light);
        assert_eq!(resolve_mode("dark"), Mode::Dark);
    }
}
