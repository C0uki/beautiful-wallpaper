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
    /// Topmost and on screen from the start, like the bar, but covering the
    /// whole display: the screen's own decorations. Unlike an overlay it is
    /// not shown and hidden by a `GlobalStates` flag, because it is not
    /// something the user opens.
    Chrome,
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

/// The settings screen. Nearly the whole display, and it takes focus: it is
/// a form with a search box at the top of it.
pub const SETTINGS: Surface = Surface {
    label: "settings",
    page: "settings.html",
    layer: Layer::Overlay,
    size: Some((1.0, 1.0)),
};

/// The first-run screen. Full screen and over everything, because on a fresh
/// install there is nothing else on screen worth seeing past it — the bar and
/// the dock have not been set up yet.
pub const WIZARD: Surface = Surface {
    label: "wizard",
    page: "wizard.html",
    layer: Layer::Overlay,
    size: Some((1.0, 1.0)),
};

/// The floating overlay's canvas. Covers the screen; its input region is cut
/// down to the pinned widgets whenever the overlay itself is shut, so it is
/// deliberately not click-through — the region is the mask.
pub const OVERLAY: Surface = Surface {
    label: "overlay",
    page: "overlay.html",
    layer: Layer::Overlay,
    size: Some((1.0, 1.0)),
};

/// The half of the overlay that never takes the pointer: pinned widgets that
/// are meant to be seen through, which in practice means the crosshair. A
/// separate window because a region masks drawing and input together, and this
/// half has to be drawn without being clickable.
pub const OVERLAY_PINNED: Surface = Surface {
    label: "overlayPinned",
    page: "overlayPinned.html",
    layer: Layer::Overlay,
    size: Some((1.0, 1.0)),
};

/// The screen's decorations: fake rounded corners and the frame. Covers the
/// whole display and is click-through everywhere, because none of it is
/// something to press.
pub const SCREEN_CHROME: Surface = Surface {
    label: "screenChrome",
    page: "screenChrome.html",
    layer: Layer::Chrome,
    size: Some((1.0, 1.0)),
};

/// The hot corners. Also covers the whole display, but it is emphatically not
/// click-through: what keeps it out of the way is its window region, which is
/// cut down to the corner strips so every click outside them lands on whatever
/// is underneath.
pub const HOT_CORNERS: Surface = Surface {
    label: "hotCorners",
    page: "hotCorners.html",
    layer: Layer::Chrome,
    size: Some((1.0, 1.0)),
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
    SCREEN_CHROME,
    HOT_CORNERS,
    OVERLAY,
    OVERLAY_PINNED,
    SETTINGS,
    WIZARD,
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
        "settingsOpen" => Some(SETTINGS.label),
        "wizardOpen" => Some(WIZARD.label),
        // `overlayOpen` is deliberately absent: the overlay stays on screen
        // after the flag clears if anything on it was pinned, so only
        // `services::overlay::apply` can decide whether its windows are up.
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

    // The overlay is not in that table, and every path that flips a flag comes
    // through here — the commands, the hotkey and the CLI alike — so this is
    // the one place its two windows can be kept honest.
    crate::services::overlay::apply(app);
}

/// Holds the bar's app-bar registration for the life of the process.
///
/// Reserving screen space lasts until it is given back, so this must outlive
/// `setup`: dropping it releases the edge, and losing it without dropping would
/// leave the work area shrunk after the shell exits.
#[derive(Default)]
pub struct Reservations {
    /// One per bar window. `perMonitor` makes this as many as there are
    /// monitors, and every one of them has to be given back.
    #[cfg(windows)]
    bars: Mutex<Vec<crate::platform::win::AppBar>>,
    #[cfg(not(windows))]
    _unused: Mutex<()>,
}

/// Gives back the screen edge the bar reserved.
///
/// The reservation lasts until it is released, and Tauri's exit path does not
/// unwind, so without this a work area shrunk for the bar stays shrunk after
/// the shell is gone — every maximised window on the machine stopping short of
/// an edge nothing is on any more.
pub fn release_reservations(app: &AppHandle) {
    #[cfg(windows)]
    if let Some(held) = app.try_state::<Reservations>() {
        drop(std::mem::take(&mut *held.bars.lock()));
    }
    #[cfg(not(windows))]
    let _ = app;
}

/// One screen a surface can be put on: where its origin is, and how big it is.
///
/// Logical rather than physical throughout, because that is what the window
/// builder and every geometry function take. A second monitor can be at a
/// different scale factor, so the division has to happen per screen rather
/// than once against the primary one.
struct Screen {
    /// Empty for the primary monitor, which keeps its plain `bar` label and so
    /// its window through a `per_monitor` change.
    device: String,
    origin: (f64, f64),
    size: (f64, f64),
}

/// Creates a surface's windows if they do not exist yet, and layers them.
///
/// One window, except for a bar asked to appear on every monitor.
pub fn ensure(app: &AppHandle, surface: &Surface) -> tauri::Result<()> {
    let config = app.state::<AppState>().config();

    for screen in screens_for(app, surface, &config) {
        let label = bar_label(surface.label, &screen.device);
        if app.get_webview_window(&label).is_some() {
            continue;
        }

        // An auto-hiding surface starts hidden; the pointer reaching its strip
        // is what reveals it, through `set_revealed`.
        let (x, y, width, height) = geometry(surface, &config, screen.size, false);

        let mut builder =
            WebviewWindowBuilder::new(app, &label, WebviewUrl::App(surface.page.into()))
                .title("beautiful-wallpaper")
                .decorations(false)
                .transparent(true)
                .shadow(false)
                .resizable(false)
                .position(x + screen.origin.0, y + screen.origin.1)
                .inner_size(width, height)
                // The shell's windows do not belong in Alt-Tab or on the taskbar.
                .skip_taskbar(true);

        builder = match surface.layer {
            Layer::Background => builder.focused(false).always_on_bottom(true),
            // On screen from the start, and never taking the focus off whatever
            // the user is actually working in.
            Layer::Bar | Layer::Chrome => builder.focused(false).always_on_top(true),
            // Overlays start hidden and are shown by their `GlobalStates` flag.
            Layer::Overlay => builder.always_on_top(true).visible(false),
        };

        let window = builder.build()?;
        apply_layer(app, &window, surface.layer, &config, &screen.device);
    }
    Ok(())
}

/// The screens a surface gets a window on.
///
/// The primary one, unless this is the bar and `bar.perMonitor` is set — then
/// every monitor, the primary one first so it keeps the plain `bar` label.
fn screens_for(app: &AppHandle, surface: &Surface, config: &bw_core::Config) -> Vec<Screen> {
    let primary = || {
        vec![Screen {
            device: String::new(),
            origin: (0.0, 0.0),
            size: primary_screen(app),
        }]
    };

    if surface.layer != Layer::Bar || !config.bar.per_monitor {
        return primary();
    }

    let Ok(monitors) = app.available_monitors() else {
        return primary();
    };

    let mut screens: Vec<Screen> = monitors
        .iter()
        .map(|monitor| {
            let scale = monitor.scale_factor().max(f64::MIN_POSITIVE);
            let position = monitor.position();
            let size = monitor.size();
            Screen {
                device: monitor.name().cloned().unwrap_or_default(),
                origin: (f64::from(position.x) / scale, f64::from(position.y) / scale),
                size: (
                    f64::from(size.width) / scale,
                    f64::from(size.height) / scale,
                ),
            }
        })
        .collect();

    if screens.is_empty() {
        return primary();
    }

    // The primary monitor is the one at the origin: Windows lays every other
    // monitor out relative to it, so this holds however they are arranged.
    if let Some(index) = screens
        .iter()
        .position(|screen| screen.origin == (0.0, 0.0))
    {
        screens[index].device = String::new();
        screens.swap(0, index);
    }
    screens
}

/// The window label a surface takes on a given monitor.
///
/// The primary monitor's bar keeps the bare label, so turning `perMonitor` on
/// and off again does not orphan the window every other part of the shell has
/// always known by that name. Tauri only accepts alphanumerics, `-`, `/`, `:`
/// and `_` in a label, and a device name is `\\.\DISPLAY2`, so everything else
/// is dropped rather than substituted — two monitors cannot produce the same
/// name to begin with.
fn bar_label(label: &str, device: &str) -> String {
    if device.is_empty() {
        return label.to_owned();
    }
    let cleaned: String = device
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
        .collect();
    if cleaned.is_empty() {
        return label.to_owned();
    }
    format!("{label}-{cleaned}")
}

/// Where a surface sits, hidden or revealed.
///
/// One entry point because two callers need the same answer: `ensure` when it
/// creates the window, and `set_revealed` when the pointer arrives at an
/// auto-hiding surface's strip and it has to come back.
fn geometry(
    surface: &Surface,
    config: &bw_core::Config,
    screen: (f64, f64),
    revealed: bool,
) -> (f64, f64, f64, f64) {
    match surface.layer {
        Layer::Background | Layer::Chrome => (0.0, 0.0, screen.0, screen.1),
        Layer::Bar => bar_geometry(config, screen, revealed),
        Layer::Overlay => overlay_geometry(surface, config, screen, revealed),
    }
}

/// Moves an auto-hiding surface between its hidden and revealed positions.
///
/// The window has to move, rather than the page sliding its own content: it is
/// not click-through, so a window left covering its whole band would swallow
/// every click meant for what is behind it, and a window parked off the edge
/// cannot bring content back on screen no matter what its CSS does. This is one
/// `SetWindowPos` per transition, not per frame — the slide itself is still the
/// page's transition, running in the compositor.
pub fn set_revealed(app: &AppHandle, label: &str, revealed: bool) {
    let Some(surface) = ALL
        .iter()
        .find(|surface| surface.label == surface_of(label))
    else {
        return;
    };
    let Some(window) = app.get_webview_window(label) else {
        return;
    };

    // The screen this window is on, which for a `perMonitor` bar is not the
    // primary one — hiding it against the primary monitor's height would send
    // it off the wrong edge, or off no edge at all.
    let (origin, screen) = match window.current_monitor() {
        Ok(Some(monitor)) => {
            let scale = monitor.scale_factor().max(f64::MIN_POSITIVE);
            let position = monitor.position();
            let size = monitor.size();
            (
                (f64::from(position.x) / scale, f64::from(position.y) / scale),
                (
                    f64::from(size.width) / scale,
                    f64::from(size.height) / scale,
                ),
            )
        }
        _ => ((0.0, 0.0), primary_screen(app)),
    };

    let config = app.state::<AppState>().config();
    let (x, y, _, _) = geometry(surface, &config, screen, revealed);

    let position = tauri::LogicalPosition::new(x + origin.0, y + origin.1);
    if let Err(error) = window.set_position(position) {
        tracing::warn!(%error, surface = label, "could not move a surface to its hover position");
    }
}

/// The surface a window label is an instance of.
///
/// `perMonitor` gives every bar but the primary one a label like
/// `bar-DISPLAY2`; no surface label contains a dash of its own, which is what
/// makes the suffix separable at all.
fn surface_of(label: &str) -> &str {
    label.split_once('-').map_or(label, |(base, _)| base)
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
    revealed: bool,
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
        let hidden = if config.dock.auto_hide && !config.dock.pinned_on_startup && !revealed {
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
fn bar_geometry(
    config: &bw_core::Config,
    screen: (f64, f64),
    revealed: bool,
) -> (f64, f64, f64, f64) {
    let thickness = f64::from(config.bar.height);
    let (screen_width, screen_height) = screen;

    // Pushed off its own edge while hidden, all but the strip the pointer has
    // to reach. The dock hides the same way, so it is the dock's arithmetic —
    // the sign is what differs, because the bar can be on any of four edges
    // while the dock is only ever on the bottom.
    let hidden = if config.bar.auto_hide && !revealed {
        bw_core::dock::hidden_offset(thickness, f64::from(config.bar.hover_region_height))
    } else {
        0.0
    };

    if config.bar.vertical {
        // A vertical bar is anchored left unless it is configured to the far
        // side, which `bar.bottom` doubles as in vertical mode — the same
        // overload the original's config uses.
        let x = if config.bar.bottom {
            screen_width - thickness + hidden
        } else {
            -hidden
        };
        (x, 0.0, thickness, screen_height)
    } else {
        let y = if config.bar.bottom {
            screen_height - thickness + hidden
        } else {
            -hidden
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
    device: &str,
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
        Layer::Bar | Layer::Overlay | Layer::Chrome => WinLayer::Overlay,
    };

    unsafe {
        if let Err(error) = win::set_layer(hwnd, target) {
            tracing::warn!(%error, "could not place the surface on its layer");
        }

        // The decorations are drawn over everything and pressed by nobody. The
        // hot corners are the exception: they are masked by their window
        // region instead, which is applied once the strips are known.
        if layer == Layer::Chrome && window.label() != HOT_CORNERS.label {
            win::set_click_through(hwnd, true);
        }
    }

    // An auto-hiding bar must not reserve its edge, whatever `reserve_space`
    // says: the reservation would hold a strip open that no window may use and
    // the bar is not in, which is the opposite of what hiding it is for.
    if layer != Layer::Bar || !config.bar.reserve_space || config.bar.auto_hide {
        return;
    }

    let edge = match (config.bar.vertical, config.bar.bottom) {
        (false, false) => Edge::Top,
        (false, true) => Edge::Bottom,
        (true, false) => Edge::Left,
        (true, true) => Edge::Right,
    };

    // The bar reserves an edge of the screen it is actually on. `device` is
    // empty for the primary monitor, which is also the only screen there is
    // unless `perMonitor` is set.
    let monitors = win::monitors();
    let Some(monitor) = monitors
        .iter()
        .find(|monitor| !device.is_empty() && monitor.name == device)
        .or_else(|| monitors.iter().find(|monitor| monitor.primary))
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
            app.state::<Reservations>().bars.lock().push(bar);
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
    _device: &str,
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
            bar_geometry(&config, (1920.0, 1080.0), false),
            (0.0, 0.0, 1920.0, f64::from(config.bar.height))
        );
    }

    #[test]
    fn a_bottom_bar_sits_on_the_bottom_edge() {
        let mut config = Config::default();
        config.bar.bottom = true;
        let (x, y, width, height) = bar_geometry(&config, (1920.0, 1080.0), false);
        assert_eq!((x, width), (0.0, 1920.0));
        assert_eq!(y + height, 1080.0);
    }

    #[test]
    fn a_vertical_bar_spans_the_height() {
        let mut config = Config::default();
        config.bar.vertical = true;
        let (x, y, width, height) = bar_geometry(&config, (1920.0, 1080.0), false);
        assert_eq!((x, y, height), (0.0, 0.0, 1080.0));
        assert_eq!(width, f64::from(config.bar.height));

        // In vertical mode `bottom` means the far side.
        config.bar.bottom = true;
        let (x, _, width, _) = bar_geometry(&config, (1920.0, 1080.0), false);
        assert_eq!(x + width, 1920.0);
    }

    /// Every edge has to hide *outwards*. A sign the wrong way round puts the
    /// bar a screen's width into the desktop instead of off its own edge, and
    /// nothing else in the shell would notice.
    #[test]
    fn an_auto_hiding_bar_leaves_only_its_hover_strip_on_screen() {
        let mut config = Config::default();
        config.bar.auto_hide = true;
        let strip = f64::from(config.bar.hover_region_height);
        let thickness = f64::from(config.bar.height);
        let screen = (1920.0, 1080.0);

        let (_, y, _, height) = bar_geometry(&config, screen, false);
        assert_eq!(y + height, strip, "a top bar hides upwards");

        config.bar.bottom = true;
        let (_, y, _, _) = bar_geometry(&config, screen, false);
        assert_eq!(screen.1 - y, strip, "a bottom bar hides downwards");

        config.bar.vertical = true;
        config.bar.bottom = false;
        let (x, _, width, _) = bar_geometry(&config, screen, false);
        assert_eq!(x + width, strip, "a left bar hides leftwards");

        config.bar.bottom = true;
        let (x, _, _, _) = bar_geometry(&config, screen, false);
        assert_eq!(screen.0 - x, strip, "a right bar hides rightwards");

        // The strip is what makes an auto-hiding bar reachable at all, so it
        // survives a configuration that asks for none.
        config.bar.hover_region_height = 0;
        let (x, _, _, _) = bar_geometry(&config, screen, false);
        assert_eq!(screen.0 - x, 1.0);

        // A hover region bigger than the bar cannot hide it further than flush.
        config.bar.hover_region_height = thickness as u32 * 2;
        let (x, _, _, _) = bar_geometry(&config, screen, false);
        assert_eq!(screen.0 - x, thickness);
    }

    #[test]
    fn a_bar_that_is_not_hiding_is_where_it_always_was() {
        let mut config = Config::default();
        config.bar.auto_hide = false;
        config.bar.hover_region_height = 40;
        assert_eq!(
            bar_geometry(&config, (1920.0, 1080.0), false),
            (0.0, 0.0, 1920.0, f64::from(config.bar.height))
        );
    }

    /// `surface_of` splits on the first dash, so a surface whose own label had
    /// one would resolve to something shorter than itself and its window would
    /// stop being found at all.
    #[test]
    fn no_surface_label_contains_the_character_that_separates_a_monitor() {
        for surface in ALL {
            assert!(!surface.label.contains('-'), "{}", surface.label);
            assert_eq!(surface_of(surface.label), surface.label);
        }
    }

    #[test]
    fn a_per_monitor_label_names_its_monitor_and_still_resolves_to_its_surface() {
        // Tauri accepts only alphanumerics, `-`, `/`, `:` and `_` in a label,
        // and a Windows device name is `\\.\DISPLAY2`.
        let label = bar_label(BAR.label, r"\\.\DISPLAY2");
        assert_eq!(label, "bar-DISPLAY2");
        assert!(label
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "-/:_".contains(c)));
        assert_eq!(surface_of(&label), BAR.label);
    }

    /// The primary monitor keeps the bare label so that turning `perMonitor`
    /// on and off does not leave the window every other part of the shell
    /// knows as `bar` orphaned under a different name.
    #[test]
    fn the_primary_monitor_keeps_the_plain_label() {
        assert_eq!(bar_label(BAR.label, ""), "bar");
        // A device name with nothing Tauri would accept falls back rather than
        // producing `bar-`, which is a label no monitor could be read out of.
        assert_eq!(bar_label(BAR.label, r"\\.\"), "bar");
    }
}
