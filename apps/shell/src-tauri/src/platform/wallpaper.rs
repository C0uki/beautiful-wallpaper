//! Setting the desktop wallpaper, and pushing the palette into Windows itself.
//!
//! This replaces the parts of `switchwall.sh` that talk to the desktop: the
//! wallpaper itself (`IDesktopWallpaper`, which is per-monitor) and the OS accent
//! colour and light/dark preference (`gsettings` upstream, the registry here).

use windows::core::{Result, HSTRING, PCWSTR};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_APARTMENTTHREADED,
};
use windows::Win32::UI::Shell::{DesktopWallpaper, IDesktopWallpaper, DESKTOP_WALLPAPER_POSITION};

/// How the image is fitted to the monitor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fit {
    Fill,
    Fit,
    Stretch,
    Center,
    Span,
}

impl Fit {
    fn as_position(self) -> DESKTOP_WALLPAPER_POSITION {
        // DWPOS_CENTER = 0, TILE = 1, STRETCH = 2, FIT = 3, FILL = 4, SPAN = 5.
        DESKTOP_WALLPAPER_POSITION(match self {
            Fit::Center => 0,
            Fit::Stretch => 2,
            Fit::Fit => 3,
            Fit::Fill => 4,
            Fit::Span => 5,
        })
    }
}

/// COM has to be initialised on any thread that talks to `IDesktopWallpaper`.
/// Calling this twice on one thread is harmless — the second call reports
/// `RPC_E_CHANGED_MODE` or `S_FALSE`, neither of which is a failure here.
fn ensure_com() {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
    }
}

/// Sets the wallpaper on every monitor, or on one when `monitor` is given.
///
/// The shell renders its own wallpaper in the WorkerW layer, but setting the
/// real one still matters: it is what the user sees before the shell starts, on
/// the lock screen preview, and in the Windows settings UI.
pub fn set(path: &str, monitor: Option<&str>, fit: Fit) -> Result<()> {
    ensure_com();
    unsafe {
        let desktop: IDesktopWallpaper = CoCreateInstance(&DesktopWallpaper, None, CLSCTX_ALL)?;
        desktop.SetPosition(fit.as_position())?;

        let image = HSTRING::from(path);
        match monitor {
            Some(id) => desktop.SetWallpaper(&HSTRING::from(id), &image)?,
            // A null monitor id means "all of them".
            None => desktop.SetWallpaper(PCWSTR::null(), &image)?,
        }
    }
    Ok(())
}

/// Reads back the wallpaper Windows currently has for a monitor.
pub fn current(monitor: Option<&str>) -> Result<String> {
    ensure_com();
    unsafe {
        let desktop: IDesktopWallpaper = CoCreateInstance(&DesktopWallpaper, None, CLSCTX_ALL)?;
        let path = match monitor {
            Some(id) => desktop.GetWallpaper(&HSTRING::from(id))?,
            None => desktop.GetWallpaper(PCWSTR::null())?,
        };
        Ok(path.to_string()?)
    }
}

/// Pushes the generated palette into Windows' own theming.
///
/// The registry is the only interface Windows offers for this. Explorer picks up
/// the light/dark switch on its own; the accent colour needs a broadcast that
/// the caller is free to skip, since most apps re-read it lazily anyway.
pub fn sync_system_theme(accent_rgb: (u8, u8, u8), dark: bool) -> std::result::Result<(), String> {
    let personalize = windows_registry::CURRENT_USER
        .create(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize")
        .map_err(|error| format!("could not open the Personalize key: {error}"))?;

    let light = u32::from(!dark);
    personalize
        .set_u32("AppsUseLightTheme", light)
        .and_then(|()| personalize.set_u32("SystemUsesLightTheme", light))
        .map_err(|error| format!("could not write the theme preference: {error}"))?;

    // DWM stores the accent as 0x00BBGGRR, not RGB.
    let (r, g, b) = accent_rgb;
    let bgr = u32::from(b) << 16 | u32::from(g) << 8 | u32::from(r);

    let dwm = windows_registry::CURRENT_USER
        .create(r"Software\Microsoft\Windows\DWM")
        .map_err(|error| format!("could not open the DWM key: {error}"))?;
    dwm.set_u32("AccentColor", bgr)
        .and_then(|()| dwm.set_u32("ColorizationColor", bgr))
        .map_err(|error| format!("could not write the accent colour: {error}"))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fit_maps_to_the_documented_positions() {
        assert_eq!(Fit::Center.as_position().0, 0);
        assert_eq!(Fit::Stretch.as_position().0, 2);
        assert_eq!(Fit::Fit.as_position().0, 3);
        assert_eq!(Fit::Fill.as_position().0, 4);
        assert_eq!(Fit::Span.as_position().0, 5);
    }
}
