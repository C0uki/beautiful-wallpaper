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

/// The volume and brightness readout. Small, transient, never focused.
pub const OSD: Surface = Surface {
    label: "osd",
    page: "osd.html",
    layer: Layer::Overlay,
    size: Some((0.24, 0.09)),
};

/// The toast stack. Covers its corner of the screen and passes clicks through
/// everywhere a toast is not.
pub const NOTIFICATIONS: Surface = Surface {
    label: "notifications",
    page: "notifications.html",
    layer: Layer::Overlay,
    size: Some((0.28, 0.85)),
};

/// The control centre. Full height along the right edge, and unlike the two
/// transient overlays it does take focus — the user opened it deliberately and
/// will want to type into its search and to-do fields.
pub const SIDEBAR_RIGHT: Surface = Surface {
    label: "sidebarRight",
    page: "sidebarRight.html",
    layer: Layer::Overlay,
    size: Some((0.26, 1.0)),
};

/// The search overlay. The whole screen, and it takes focus: it exists to be
/// typed into, and the default centring gives a full-size surface the origin
/// without a branch of its own.
pub const OVERVIEW: Surface = Surface {
    label: "overview",
    page: "overview.html",
    layer: Layer::Overlay,
    size: Some((1.0, 1.0)),
};

/// The session screen. The whole screen, and it takes focus so the keyboard
/// can reach the buttons.
pub const SESSION: Surface = Surface {
    label: "session",
    page: "session.html",
    layer: Layer::Overlay,
    size: Some((1.0, 1.0)),
};

/// The region picker. The whole screen, drawn on a frozen copy of it, and it
/// takes focus so that Escape and Enter reach it rather than whatever is
/// behind.
pub const REGION_SELECT: Surface = Surface {
    label: "regionSelect",
    page: "regionSelect.html",
    layer: Layer::Overlay,
    size: Some((1.0, 1.0)),
};

/// The desktop menu. The whole screen — the menu itself is a small panel
/// drawn on a transparent sheet — and it takes focus so the arrow keys and
/// Escape reach it rather than whatever is behind.
pub const DESKTOP_MENU: Surface = Surface {
    label: "desktopMenu",
    page: "desktopMenu.html",
    layer: Layer::Overlay,
    size: Some((1.0, 1.0)),
};

/// The drop shelf. A tall panel against one edge, and it takes focus: files
/// are dragged on to it and off it, and a window that cannot be clicked into
/// cannot be dragged out of either.
pub const SHELF: Surface = Surface {
    label: "shelf",
    page: "shelf.html",
    layer: Layer::Overlay,
    size: Some((0.2, 1.0)),
};

/// The dock. Full width along the bottom, and never focused: clicking an icon
/// should put the user in *that* application, not in the dock.
pub const DOCK: Surface = Surface {
    label: "dock",
    page: "dock.html",
    layer: Layer::Overlay,
    size: Some((1.0, 0.12)),
};

/// The left panel: translator and media.
pub const SIDEBAR_LEFT: Surface = Surface {
    label: "sidebarLeft",
    page: "sidebarLeft.html",
    layer: Layer::Overlay,
    size: Some((0.26, 1.0)),
};

pub const ALL: &[Surface] = &[
    BACKGROUND,
    BAR,
    WALLPAPER_SELECTOR,
    OSD,
    NOTIFICATIONS,
    SIDEBAR_RIGHT,
    SIDEBAR_LEFT,
    DOCK,
    OVERVIEW,
    REGION_SELECT,
    SESSION,
    DESKTOP_MENU,
    SHELF,
];

/// Which surface a `GlobalStates` flag governs.
///
/// Without this the flags were only ever a message to the frontend, and the
/// overlay windows — created hidden — had no path to being shown at all.
pub fn surface_for_flag(flag: &str) -> Option<&'static str> {
    match flag {
        "wallpaperSelectorOpen" => Some(WALLPAPER_SELECTOR.label),
        "sidebarRightOpen" => Some(SIDEBAR_RIGHT.label),
        "sidebarLeftOpen" => Some(SIDEBAR_LEFT.label),
        "overviewOpen" => Some(OVERVIEW.label),
        "regionSelectOpen" => Some(REGION_SELECT.label),
        "sessionOpen" => Some(SESSION.label),
        "desktopMenuOpen" => Some(DESKTOP_MENU.label),
        "shelfOpen" => Some(SHELF.label),
        _ => None,
    }
}

/// Applies every flag to its surface.
pub fn apply_states(app: &AppHandle, states: &crate::state::GlobalStates) {
    let Ok(value) = serde_json::to_value(states) else {
        return;
    };
    let Some(flags) = value.as_object() else {
        return;
    };

    for (flag, open) in flags {
        let Some(label) = surface_for_flag(flag) else {
            continue;
        };
        if let Err(error) = set_visible(app, label, open.as_bool().unwrap_or(false)) {
            tracing::warn!(%error, surface = label, "could not change a surface's visibility");
        }
    }
}

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
        Layer::Overlay => overlay_geometry(surface, &config, screen),
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

/// Where an overlay sits.
///
/// Most overlays are centred, but the two transient ones are not: the readout
/// hugs the edge opposite the bar, and the toasts sit in the corner the user
/// chose. Both keep clear of the bar rather than sliding under it.
fn overlay_geometry(
    surface: &Surface,
    config: &bw_core::Config,
    screen: (f64, f64),
) -> (f64, f64, f64, f64) {
    let (fraction_w, fraction_h) = surface.size.unwrap_or((0.5, 0.5));
    let (width, height) = (screen.0 * fraction_w, screen.1 * fraction_h);
    let bar = if config.bar.enable {
        f64::from(config.bar.height)
    } else {
        0.0
    };
    let margin = 8.0;

    if surface.label == OSD.label {
        // The readout goes to the top unless the bar is there, in which case it
        // goes below it — the original does the same.
        let y = if config.bar.bottom {
            margin
        } else {
            bar + margin
        };
        let y = if config.osd.position == "bottom" {
            screen.1
                - height
                - if config.bar.bottom {
                    bar + margin
                } else {
                    margin
                }
        } else {
            y
        };
        return ((screen.0 - width) / 2.0, y, width, height);
    }

    if surface.label == DOCK.label {
        // As wide as its content, centred, and pushed off the bottom while
        // hidden — all but the hover strip, which is what the pointer has to
        // reach to bring it back.
        let height = f64::from(config.dock.height) + margin * 2.0;
        let hidden = if config.dock.auto_hide && !config.dock.pinned_on_startup {
            bw_core::dock::hidden_offset(height, f64::from(config.dock.hover_region_height))
        } else {
            0.0
        };
        let bottom_bar = if config.bar.enable && !config.bar.vertical && config.bar.bottom {
            bar
        } else {
            0.0
        };
        return (
            0.0,
            screen.1 - height - bottom_bar + hidden,
            screen.0,
            height,
        );
    }

    if surface.label == SIDEBAR_LEFT.label {
        let width = screen.0 * config.sidebar.left.width;
        let vertical_bar = config.bar.enable && config.bar.vertical;
        let horizontal_bar = if config.bar.enable && !config.bar.vertical {
            bar
        } else {
            0.0
        };

        let left_inset = if vertical_bar && !config.bar.bottom {
            bar
        } else {
            0.0
        };
        let top = if horizontal_bar > 0.0 && !config.bar.bottom {
            horizontal_bar
        } else {
            0.0
        };
        let bottom = if horizontal_bar > 0.0 && config.bar.bottom {
            horizontal_bar
        } else {
            0.0
        };

        return (
            left_inset + margin,
            top + margin,
            width,
            (screen.1 - top - bottom - margin * 2.0).max(1.0),
        );
    }

    if surface.label == SHELF.label {
        // Against whichever edge was asked for, and clear of the bar rather
        // than sliding under it — the same reasoning as the sidebars.
        let width = screen.0 * config.shelf.width;
        let vertical_bar = config.bar.enable && config.bar.vertical;
        let horizontal_bar = if config.bar.enable && !config.bar.vertical {
            bar
        } else {
            0.0
        };
        let top = if horizontal_bar > 0.0 && !config.bar.bottom {
            horizontal_bar
        } else {
            0.0
        };
        let bottom = if horizontal_bar > 0.0 && config.bar.bottom {
            horizontal_bar
        } else {
            0.0
        };

        let x = if config.shelf.edge == "left" {
            let inset = if vertical_bar && !config.bar.bottom {
                bar
            } else {
                0.0
            };
            inset + margin
        } else {
            let inset = if vertical_bar && config.bar.bottom {
                bar
            } else {
                0.0
            };
            screen.0 - width - inset - margin
        };

        return (
            x,
            top + margin,
            width,
            (screen.1 - top - bottom - margin * 2.0).max(1.0),
        );
    }

    if surface.label == SIDEBAR_RIGHT.label {
        // Pinned to the right edge and as tall as the work area allows, kept
        // clear of the bar rather than sliding under it.
        let width = screen.0 * config.sidebar.width;
        let vertical_bar = config.bar.enable && config.bar.vertical;
        let horizontal_bar = if config.bar.enable && !config.bar.vertical {
            bar
        } else {
            0.0
        };

        let right_inset = if vertical_bar && config.bar.bottom {
            bar
        } else {
            0.0
        };
        let top = if horizontal_bar > 0.0 && !config.bar.bottom {
            horizontal_bar
        } else {
            0.0
        };
        let bottom = if horizontal_bar > 0.0 && config.bar.bottom {
            horizontal_bar
        } else {
            0.0
        };

        return (
            screen.0 - width - right_inset - margin,
            top + margin,
            width,
            (screen.1 - top - bottom - margin * 2.0).max(1.0),
        );
    }

    if surface.label == NOTIFICATIONS.label {
        let position = config.notifications.position.as_str();
        let x = if position.ends_with("left") {
            margin
        } else if position.ends_with("center") {
            (screen.0 - width) / 2.0
        } else {
            screen.0 - width - margin
        };
        let y = if position.starts_with("bottom") {
            screen.1
                - height
                - if config.bar.bottom {
                    bar + margin
                } else {
                    margin
                }
        } else if config.bar.bottom {
            margin
        } else {
            bar + margin
        };
        return (x, y, width, height);
    }

    (
        (screen.0 - width) / 2.0,
        (screen.1 - height) / 2.0,
        width,
        height,
    )
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
        // The readout and the toasts never do — taking focus from whatever the
        // user is typing into would be worse than the information is worth.
        if takes_focus(label) {
            window.set_focus()?;
        }
    } else {
        window.hide()?;
    }
    Ok(())
}

/// Whether showing this surface should also focus it.
fn takes_focus(label: &str) -> bool {
    // The dock joins the two transient overlays here: clicking an icon should
    // put the user in the application they picked, and a dock that grabbed
    // focus first would take it straight back off them.
    !matches!(label, l if l == OSD.label || l == NOTIFICATIONS.label || l == DOCK.label)
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
