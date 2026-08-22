//! Creating the shell's windows and putting them on the right layer.
//!
//! Each surface is one frameless, transparent webview. What distinguishes them
//! is only where Win32 puts them: the background under the desktop icons, the
//! picker as a topmost overlay.

use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

/// A surface's identity: its label, its page, and how it should be layered.
pub struct Surface {
    pub label: &'static str,
    pub page: &'static str,
    pub layer: Layer,
    /// Fraction of the monitor the window covers, for the overlay surfaces.
    pub size: Option<(f64, f64)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layer {
    /// Below the icons, above the wallpaper.
    Background,
    /// Topmost, never focused.
    Overlay,
}

pub const BACKGROUND: Surface = Surface {
    label: "background",
    page: "background.html",
    layer: Layer::Background,
    size: None,
};

pub const WALLPAPER_SELECTOR: Surface = Surface {
    label: "wallpaperSelector",
    page: "wallpaperSelector.html",
    layer: Layer::Overlay,
    size: Some((0.62, 0.7)),
};

pub const ALL: &[Surface] = &[BACKGROUND, WALLPAPER_SELECTOR];

/// Creates a surface's window if it does not exist yet, and layers it.
pub fn ensure(app: &AppHandle, surface: &Surface) -> tauri::Result<()> {
    if app.get_webview_window(surface.label).is_some() {
        return Ok(());
    }

    let monitor = app.primary_monitor()?;
    let (screen_width, screen_height) = monitor
        .as_ref()
        .map(|monitor| {
            let size = monitor.size();
            let scale = monitor.scale_factor();
            (
                f64::from(size.width) / scale,
                f64::from(size.height) / scale,
            )
        })
        .unwrap_or((1920.0, 1080.0));

    let (width, height) = match surface.size {
        Some((w, h)) => (screen_width * w, screen_height * h),
        None => (screen_width, screen_height),
    };

    let mut builder =
        WebviewWindowBuilder::new(app, surface.label, WebviewUrl::App(surface.page.into()))
            .title("beautiful-wallpaper")
            .decorations(false)
            .transparent(true)
            .shadow(false)
            .resizable(false)
            .inner_size(width, height)
            // The shell's windows do not belong in Alt-Tab or on the taskbar.
            .skip_taskbar(true);

    builder = match surface.layer {
        Layer::Background => builder
            .position(0.0, 0.0)
            .focused(false)
            .always_on_bottom(true),
        Layer::Overlay => builder
            .center()
            .always_on_top(true)
            .visible(surface.label != WALLPAPER_SELECTOR.label),
    };

    let window = builder.build()?;
    apply_layer(&window, surface.layer);
    Ok(())
}

#[cfg(windows)]
fn apply_layer(window: &tauri::WebviewWindow, layer: Layer) {
    use crate::platform::win::{self, Layer as WinLayer};
    use windows::Win32::Foundation::HWND;

    let Ok(handle) = window.hwnd() else {
        tracing::warn!("a surface has no window handle yet");
        return;
    };
    let hwnd = HWND(handle.0);

    let target = match layer {
        Layer::Background => WinLayer::Wallpaper,
        Layer::Overlay => WinLayer::Overlay,
    };

    unsafe {
        if let Err(error) = win::set_layer(hwnd, target) {
            tracing::warn!(%error, "could not place the surface on its layer");
        }
    }
}

#[cfg(not(windows))]
fn apply_layer(_window: &tauri::WebviewWindow, _layer: Layer) {}

/// Shows or hides a surface, following a `GlobalStates` flag.
pub fn set_visible(app: &AppHandle, label: &str, visible: bool) -> tauri::Result<()> {
    let Some(window) = app.get_webview_window(label) else {
        return Ok(());
    };
    if visible {
        window.show()?;
        // An overlay only takes focus when the user opened it deliberately.
        window.set_focus()?;
    } else {
        window.hide()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_surface_has_a_distinct_label_and_page() {
        let labels: Vec<&str> = ALL.iter().map(|surface| surface.label).collect();
        let mut unique = labels.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(labels.len(), unique.len(), "duplicate surface label");

        for surface in ALL {
            assert!(surface.page.ends_with(".html"), "{}", surface.page);
        }
    }

    #[test]
    fn the_background_covers_the_screen_and_overlays_do_not() {
        assert!(BACKGROUND.size.is_none());
        let (w, h) = WALLPAPER_SELECTOR.size.expect("an overlay needs a size");
        assert!((0.0..=1.0).contains(&w) && (0.0..=1.0).contains(&h));
    }
}
