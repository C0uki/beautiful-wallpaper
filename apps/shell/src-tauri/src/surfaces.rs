//! Creating the shell's windows and putting them on the right layer.
//!
//! Each surface is one frameless, transparent webview. What distinguishes them is
//! only where Win32 puts them: the background under the desktop icons, the bar
//! along a reserved edge, the picker as a topmost overlay.

use parking_lot::Mutex;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

use crate::state::AppState;

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
    /// Along a screen edge, with the work area reserved for it.
    Bar,
    /// Topmost, never focused.
    Overlay,
}

pub const BACKGROUND: Surface = Surface {
    label: "background",
    page: "background.html",
    layer: Layer::Background,
    size: None,
};

pub const BAR: Surface = Surface {
    label: "bar",
    page: "bar.html",
    layer: Layer::Bar,
    size: None,
};

pub const WALLPAPER_SELECTOR: Surface = Surface {
    label: "wallpaperSelector",
    page: "wallpaperSelector.html",
    layer: Layer::Overlay,
    size: Some((0.62, 0.7)),
};

pub const ALL: &[Surface] = &[BACKGROUND, BAR, WALLPAPER_SELECTOR];

/// Holds the bar's app-bar registration for the life of the process.
///
/// Reserving screen space lasts until it is given back, so this must outlive
/// `setup`: dropping it releases the edge, and losing it without dropping would
/// leave the work area shrunk after the shell exits.
#[derive(Default)]
pub struct Reservations {
    #[cfg(windows)]
    bar: Mutex<Option<crate::platform::win::AppBar>>,
    #[cfg(not(windows))]
    _unused: Mutex<()>,
}

/// Creates a surface's window if it does not exist yet, and layers it.
pub fn ensure(app: &AppHandle, surface: &Surface) -> tauri::Result<()> {
    if app.get_webview_window(surface.label).is_some() {
        return Ok(());
    }

    let screen = primary_screen(app);
    let config = app.state::<AppState>().config();

    let (x, y, width, height) = match surface.layer {
        Layer::Background => (0.0, 0.0, screen.0, screen.1),
        Layer::Bar => bar_geometry(&config, screen),
        Layer::Overlay => {
            let (w, h) = surface.size.unwrap_or((0.5, 0.5));
            let (width, height) = (screen.0 * w, screen.1 * h);
            (
                (screen.0 - width) / 2.0,
                (screen.1 - height) / 2.0,
                width,
                height,
            )
        }
    };

    let mut builder =
        WebviewWindowBuilder::new(app, surface.label, WebviewUrl::App(surface.page.into()))
            .title("beautiful-wallpaper")
            .decorations(false)
            .transparent(true)
            .shadow(false)
            .resizable(false)
            .position(x, y)
            .inner_size(width, height)
            // The shell's windows do not belong in Alt-Tab or on the taskbar.
            .skip_taskbar(true);

    builder = match surface.layer {
        Layer::Background => builder.focused(false).always_on_bottom(true),
        Layer::Bar => builder.focused(false).always_on_top(true),
        // Overlays start hidden and are shown by their `GlobalStates` flag.
        Layer::Overlay => builder.always_on_top(true).visible(false),
    };

    let window = builder.build()?;
    apply_layer(app, &window, surface.layer, &config);
    Ok(())
}

/// The bar's rectangle before the shell has had a chance to negotiate it.
fn bar_geometry(config: &bw_core::Config, screen: (f64, f64)) -> (f64, f64, f64, f64) {
    let thickness = f64::from(config.bar.height);
    let (screen_width, screen_height) = screen;

    if config.bar.vertical {
        // A vertical bar is anchored left unless it is configured to the far
        // side, which `bar.bottom` doubles as in vertical mode — the same
        // overload the original's config uses.
        let x = if config.bar.bottom {
            screen_width - thickness
        } else {
            0.0
        };
        (x, 0.0, thickness, screen_height)
    } else {
        let y = if config.bar.bottom {
            screen_height - thickness
        } else {
            0.0
        };
        (0.0, y, screen_width, thickness)
    }
}

/// Logical size of the primary monitor.
fn primary_screen(app: &AppHandle) -> (f64, f64) {
    app.primary_monitor()
        .ok()
        .flatten()
        .map(|monitor| {
            let size = monitor.size();
            let scale = monitor.scale_factor();
            (
                f64::from(size.width) / scale,
                f64::from(size.height) / scale,
            )
        })
        .unwrap_or((1920.0, 1080.0))
}

#[cfg(windows)]
fn apply_layer(
    app: &AppHandle,
    window: &tauri::WebviewWindow,
    layer: Layer,
    config: &bw_core::Config,
) {
    use crate::platform::win::{self, Edge, Layer as WinLayer};
    use windows::Win32::Foundation::HWND;

    let Ok(handle) = window.hwnd() else {
        tracing::warn!("a surface has no window handle yet");
        return;
    };
    let hwnd = HWND(handle.0);

    let target = match layer {
        Layer::Background => WinLayer::Wallpaper,
        Layer::Bar | Layer::Overlay => WinLayer::Overlay,
    };

    unsafe {
        if let Err(error) = win::set_layer(hwnd, target) {
            tracing::warn!(%error, "could not place the surface on its layer");
        }
    }

    if layer != Layer::Bar || !config.bar.reserve_space {
        return;
    }

    let edge = match (config.bar.vertical, config.bar.bottom) {
        (false, false) => Edge::Top,
        (false, true) => Edge::Bottom,
        (true, false) => Edge::Left,
        (true, true) => Edge::Right,
    };

    let monitors = win::monitors();
    let Some(monitor) = monitors
        .iter()
        .find(|monitor| monitor.primary)
        .or_else(|| monitors.first())
    else {
        return;
    };

    let thickness = config.bar.height as i32;
    let reservation = unsafe { win::AppBar::register(hwnd, edge, thickness, monitor) };

    match reservation {
        Some(bar) => {
            // Windows may grant a different rectangle than the one asked for, so
            // the window follows the grant rather than the request.
            let granted = bar.granted;
            let _ = window.set_position(tauri::PhysicalPosition::new(granted.left, granted.top));
            let _ = window.set_size(tauri::PhysicalSize::new(
                (granted.right - granted.left).max(1),
                (granted.bottom - granted.top).max(1),
            ));
            *app.state::<Reservations>().bar.lock() = Some(bar);
        }
        None => tracing::warn!("the shell refused to reserve space for the bar"),
    }
}

#[cfg(not(windows))]
fn apply_layer(
    _app: &AppHandle,
    _window: &tauri::WebviewWindow,
    _layer: Layer,
    _config: &bw_core::Config,
) {
}

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
    use bw_core::Config;

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

    #[test]
    fn a_top_bar_spans_the_width_at_the_top() {
        let config = Config::default();
        assert_eq!(
            bar_geometry(&config, (1920.0, 1080.0)),
            (0.0, 0.0, 1920.0, f64::from(config.bar.height))
        );
    }

    #[test]
    fn a_bottom_bar_sits_on_the_bottom_edge() {
        let mut config = Config::default();
        config.bar.bottom = true;
        let (x, y, width, height) = bar_geometry(&config, (1920.0, 1080.0));
        assert_eq!((x, width), (0.0, 1920.0));
        assert_eq!(y + height, 1080.0);
    }

    #[test]
    fn a_vertical_bar_spans_the_height() {
        let mut config = Config::default();
        config.bar.vertical = true;
        let (x, y, width, height) = bar_geometry(&config, (1920.0, 1080.0));
        assert_eq!((x, y, height), (0.0, 0.0, 1080.0));
        assert_eq!(width, f64::from(config.bar.height));

        // In vertical mode `bottom` means the far side.
        config.bar.bottom = true;
        let (x, _, width, _) = bar_geometry(&config, (1920.0, 1080.0));
        assert_eq!(x + width, 1920.0);
    }
}
