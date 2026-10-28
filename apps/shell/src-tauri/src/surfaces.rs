//! Creating the shell's windows and putting them on the right layer.
//!
//! Each surface is one frameless, transparent webview. What distinguishes them is
//! only where Win32 puts them: the background under the desktop icons, the
//! picker as a topmost overlay. The bar, the dock, the volume readout and the
//! notification banners are not webviews at all: `bw-native` draws them, and
//! the bar's edge is reserved here.

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
    /// Topmost, never focused.
    Overlay,
    /// Topmost and on screen from the start, covering the whole display: the screen's own decorations. Unlike an overlay it is
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

pub const WALLPAPER_SELECTOR: Surface = Surface {
    label: "wallpaperSelector",
    page: "wallpaperSelector.html",
    layer: Layer::Overlay,
    size: Some((0.62, 0.7)),
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

/// The tips, as Apple's Tips app has them: what the shell can do, a card at a
/// time. Opened by the user and only then, so made on demand.
pub const TIPS: Surface = Surface {
    label: "tips",
    page: "tips.html",
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

/// The screen's decoration: the frame. Covers the whole display and is
/// click-through everywhere, because none of it is something to press. Made
/// only while there is a frame to draw (`services::chrome::emit`).
pub const SCREEN_CHROME: Surface = Surface {
    label: "screenChrome",
    page: "screenChrome.html",
    layer: Layer::Chrome,
    size: Some((1.0, 1.0)),
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
    WALLPAPER_SELECTOR,
    SIDEBAR_RIGHT,
    SIDEBAR_LEFT,
    OVERVIEW,
    REGION_SELECT,
    SESSION,
    DESKTOP_MENU,
    SHELF,
    SCREEN_CHROME,
    OVERLAY,
    OVERLAY_PINNED,
    SETTINGS,
    WIZARD,
    TIPS,
];

/// What the startup makes first: everything on screen from the start. A start
/// is over, to whoever is watching it, once these are up.
pub const FIRST: &[Surface] = &[BACKGROUND];

/// Made once the rest of the startup has run, so opening one is as quick as it
/// always was.
pub const AFTER: &[Surface] = &[
    SIDEBAR_RIGHT,
    SIDEBAR_LEFT,
    OVERVIEW,
    REGION_SELECT,
    DESKTOP_MENU,
    OVERLAY,
    OVERLAY_PINNED,
];

/// Made when opened and put away for good when closed.
///
/// Each surface is a webview of its own, tens of megabytes and seconds to make
/// on a slow machine, and these are opened rarely enough that keeping them
/// around cost more than the wait does: the settings, the first-run screen, the
/// tips, the way out of the session, the wallpaper picker and the shelf.
const ON_DEMAND: &[Surface] = &[SETTINGS, WIZARD, TIPS, SESSION, WALLPAPER_SELECTOR, SHELF];

fn on_demand(label: &str) -> Option<&'static Surface> {
    ON_DEMAND.iter().find(|surface| surface.label == label)
}

/// Queues `work` for a later turn of the event loop.
///
/// Creating a webview takes long enough that nineteen of them back to back
/// leave the main thread without a message pump for well past the five seconds
/// Windows waits before it decides a window is hung. What it puts up then is a
/// full-screen "not responding" ghost over `screenChrome` — transparent but
/// not layered, which is exactly the pair that swallows every click on the
/// desktop rather than passing it through. Spread over the loop, the thread is
/// never silent long enough to be asked.
///
/// Handed over from a thread of its own. Called on the main thread, as all of
/// the startup is, `run_on_main_thread` does not queue the work but runs it
/// there and then: the nineteen surfaces were made inside `setup`, one call
/// thirty seconds long on a slow machine, and nothing sent to a surface in
/// that time — the bar's readings, the weather — reached one until it ended.
/// From another thread it goes to the back of the loop's queue; one thread, so
/// the work keeps the order it was queued in.
pub fn later(handle: &tauri::AppHandle, work: impl FnOnce() + Send + 'static) {
    type Work = Box<dyn FnOnce() + Send>;
    static QUEUE: std::sync::OnceLock<std::sync::mpsc::Sender<Work>> = std::sync::OnceLock::new();
    let queue = QUEUE.get_or_init(|| {
        let (sender, receiver) = std::sync::mpsc::channel::<Work>();
        let handle = handle.clone();
        std::thread::Builder::new()
            .name("bw-later".to_owned())
            .spawn(move || {
                for work in receiver {
                    if let Err(error) = handle.run_on_main_thread(work) {
                        tracing::error!(%error, "could not queue a step of the startup");
                    }
                }
            })
            .expect("could not start the thread that queues the startup");
        sender
    });
    let _ = queue.send(Box::new(work));
}

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
        "tipsOpen" => Some(TIPS.label),
        // `overlayOpen` is deliberately absent: the overlay stays on screen
        // after the flag clears if anything on it was pinned, so only
        // `services::overlay::apply` can decide whether its windows are up.
        _ => None,
    }
}

/// Whether any flag governing this surface is set.
fn flagged_open(states: &crate::state::GlobalStates, label: &str) -> bool {
    let Ok(serde_json::Value::Object(flags)) = serde_json::to_value(states) else {
        return false;
    };
    flags
        .iter()
        .any(|(flag, open)| surface_for_flag(flag) == Some(label) && open.as_bool() == Some(true))
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

    // For the same reason: a click elsewhere puts a sidebar away, and this is
    // where one is known to be open.
    if let Some(dock) = app.try_state::<crate::state::DockHandle>() {
        dock.watch_clicks(states.sidebar_left_open || states.sidebar_right_open);
    }
}

/// Names any surface that is covering a monitor with nothing to let a click
/// past, once each time it starts doing it.
///
/// Which surface is on top is decided in a dozen places — flags, the overlay's
/// own bookkeeping, the capture flow, Tauri itself — so this watches the
/// result rather than any one of those paths. It only reads window state and
/// writes a log line; it does not try to fix what it finds, because hiding a
/// surface the user just opened would be its own bug.
#[cfg(windows)]
pub fn warn_about_surfaces_that_swallow_a_monitor(app: &AppHandle) {
    use std::collections::BTreeMap;
    use std::sync::{Mutex, OnceLock};
    use windows::Win32::Foundation::HWND;

    /// How many checks in a row a surface has been swallowing its monitor.
    static SPELLS: OnceLock<Mutex<BTreeMap<String, u8>>> = OnceLock::new();
    let spells = SPELLS.get_or_init(|| Mutex::new(BTreeMap::new()));
    let Ok(mut spells) = spells.lock() else {
        return;
    };

    for (label, window) in app.webview_windows() {
        // The wallpaper is meant to cover its monitor, and for the whole of the
        // second or so its webview takes to build it is doing so from the top,
        // before `set_layer` puts it under the icons. Nothing here could tell
        // that apart from the real fault, and it does not have to: when that
        // reparenting fails, `apply_layer` says so where it happens.
        if label.starts_with(BACKGROUND.label) {
            continue;
        }

        let swallowing = window.is_visible().unwrap_or(false)
            && match window.hwnd() {
                Ok(handle) => unsafe { crate::platform::win::swallows_its_monitor(HWND(handle.0)) },
                Err(_) => false,
            };

        if !swallowing {
            spells.remove(&label);
            continue;
        }

        // Two checks running, not one: a surface is briefly this shape while it
        // is being built — the wallpaper covers the screen from the moment it
        // exists and only goes under the icons once `set_layer` reparents it,
        // and creating a webview pumps messages, so this can run in between.
        // A surface that is really stuck is still stuck a second later.
        //
        // Then once per spell, not once a second: it runs on a timer, and one
        // that stays like this would otherwise repeat the line until the shell
        // is stopped.
        let seen = spells.entry(label.clone()).or_default();
        *seen = seen.saturating_add(1);
        if *seen == 2 {
            tracing::warn!(
                surface = %label,
                "this surface covers its monitor and is neither click-through nor masked,                      so every click on that screen lands on it and goes no further"
            );
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

/// One monitor a surface can be put on.
///
/// The geometry is worked out in the monitor's own logical pixels and put on
/// screen in physical ones. A second monitor can be at a different scale
/// factor, so the division has to happen per monitor rather than once against
/// the primary one, and a logical position names no one place on a desktop of
/// mixed scales.
struct Screen {
    /// Its corner, in physical pixels.
    origin: (i32, i32),
    /// Its size, in its own logical pixels.
    size: (f64, f64),
    scale: f64,
    /// Whether one of the bars is along its edge, for the overlays to keep
    /// clear of.
    has_bar: bool,
    /// How far the work area stops short of each edge — left, top, right,
    /// bottom — in logical pixels: the Windows taskbar, while it is showing,
    /// or another program's app bar.
    work_inset: (f64, f64, f64, f64),
}

impl Screen {
    fn of(monitor: &tauri::Monitor, primary: bool, config: &bw_core::Config) -> Self {
        let scale = if monitor.scale_factor() > 0.0 {
            monitor.scale_factor()
        } else {
            1.0
        };
        let size = monitor.size();
        let (at, work) = (monitor.position(), monitor.work_area());
        let inset = |gap: i32| f64::from(gap.max(0)) / scale;
        let far = |start: i32, length: u32| start + length as i32;
        Self {
            origin: (at.x, at.y),
            size: (
                f64::from(size.width) / scale,
                f64::from(size.height) / scale,
            ),
            scale,
            has_bar: config.bar.enable && (primary || config.bar.per_monitor),
            work_inset: (
                inset(work.position.x - at.x),
                inset(work.position.y - at.y),
                inset(far(at.x, size.width) - far(work.position.x, work.size.width)),
                inset(far(at.y, size.height) - far(work.position.y, work.size.height)),
            ),
        }
    }

    /// The primary monitor.
    fn primary(app: &AppHandle, config: &bw_core::Config) -> Self {
        match app.primary_monitor() {
            Ok(Some(monitor)) => Self::of(&monitor, true, config),
            // A machine that will not name its monitor still gets its
            // surfaces somewhere; 1080p is the safest guess to be wrong in the
            // small direction.
            _ => Self {
                origin: (0, 0),
                size: (1920.0, 1080.0),
                scale: 1.0,
                has_bar: config.bar.enable,
                work_inset: (0.0, 0.0, 0.0, 0.0),
            },
        }
    }

    /// The monitor the pointer is on, which is where an overlay opens: on the
    /// screen the user is looking at, rather than on the primary one whatever
    /// they were doing.
    fn under_pointer(app: &AppHandle, config: &bw_core::Config) -> Self {
        let found = app
            .cursor_position()
            .ok()
            .and_then(|at| app.monitor_from_point(at.x, at.y).ok().flatten());
        Self::or_primary(app, found, config)
    }

    /// The monitor a window is mostly on.
    fn of_window(app: &AppHandle, window: &tauri::WebviewWindow, config: &bw_core::Config) -> Self {
        Self::or_primary(app, window.current_monitor().ok().flatten(), config)
    }

    fn or_primary(
        app: &AppHandle,
        found: Option<tauri::Monitor>,
        config: &bw_core::Config,
    ) -> Self {
        let primary = app.primary_monitor().ok().flatten();
        match found {
            Some(monitor) => {
                let is_primary = primary
                    .as_ref()
                    .is_some_and(|primary| primary.position() == monitor.position());
                Self::of(&monitor, is_primary, config)
            }
            None => Self::primary(app, config),
        }
    }
}

/// Reserves a native bar's edge of its monitor — the top, or the left when it
/// is vertical — and answers the rectangle Windows granted, in physical
/// pixels.
#[cfg(windows)]
fn reserve_native_bar(
    app: &AppHandle,
    hwnd: isize,
    (x, y, width, height): bw_native::Rect,
    vertical: bool,
    thickness: i32,
) -> Option<bw_native::Rect> {
    use crate::platform::win::{self, Edge};
    use windows::Win32::Foundation::HWND;

    let monitor = win::monitors().into_iter().find(|monitor| {
        (monitor.x, monitor.y, monitor.width, monitor.height) == (x, y, width, height)
    })?;
    let edge = if vertical { Edge::Left } else { Edge::Top };
    let bar = unsafe { win::AppBar::register(HWND(hwnd as _), edge, thickness, &monitor) }?;
    let granted = bar.granted;
    // Held where the webview bar's was, so the exit gives it back the same way.
    app.state::<Reservations>().bars.lock().push(bar);
    Some((
        granted.left,
        granted.top,
        granted.right - granted.left,
        granted.bottom - granted.top,
    ))
}

#[cfg(not(windows))]
fn reserve_native_bar(
    _app: &AppHandle,
    _hwnd: isize,
    _monitor: bw_native::Rect,
    _vertical: bool,
    _thickness: i32,
) -> Option<bw_native::Rect> {
    None
}

/// Creates a surface's window if it does not exist yet, and layers it.
pub fn ensure(app: &AppHandle, surface: &Surface) -> tauri::Result<()> {
    let config = app.state::<AppState>().config();

    // Made on the primary monitor. An overlay is moved to the pointer's each
    // time it opens, and the wallpaper surface is spread over every monitor by
    // the layer it is put on.
    let screen = Screen::primary(app, &config);
    let label = if surface.label == BACKGROUND.label {
        background_label()
    } else {
        surface.label.to_owned()
    };
    if app.get_webview_window(&label).is_some() {
        return Ok(());
    }

    let (x, y, width, height) = geometry(surface, &config, &screen);

    let mut builder = WebviewWindowBuilder::new(app, &label, WebviewUrl::App(surface.page.into()))
        // Named after the surface. Nothing shows these titles — the
        // windows have no decorations and stay out of the taskbar —
        // but every tool that can look at a window shows one, and a
        // screen full of identical "beautiful-wallpaper" entries is
        // what makes "which surface is covering the desktop?" a guess.
        .title(format!("beautiful-wallpaper — {label}"))
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .resizable(false)
        .position(x, y)
        .inner_size(width, height)
        // The shell's windows do not belong in Alt-Tab or on the taskbar.
        .skip_taskbar(true)
        // None of them takes the focus off whatever the user is working
        // in, and all of this from birth rather than once built: the
        // webview takes seconds to come up, and a window visible and
        // activatable for that long is one Windows may hand the
        // foreground to. `focused` also covers the hidden ones — left at
        // Tauri's default, Wry moves focus into the webview the moment
        // it exists, handing the keyboard to a window nobody can see.
        // `set_visible` focuses the ones meant to have it, when opened.
        //
        // `focusable` is `WS_EX_NOACTIVATE`, through Tao so it survives
        // Tao rewriting the styles; `focused` is what makes Tao show
        // these with `SW_SHOWNOACTIVATE` every time, not just the first.
        .focused(false)
        .focusable(false);

    // Whether it is on screen from birth. Decided here rather than asked
    // of the window once built: Tauri can still report a window it is
    // about to show as hidden, and a page told it was hidden then stayed
    // blank until something showed it again.
    let born_visible = match surface.layer {
        Layer::Overlay => false,
        Layer::Background | Layer::Chrome => true,
    };

    builder = match surface.layer {
        Layer::Background => builder.always_on_bottom(true),
        Layer::Chrome => builder.always_on_top(true),
        // Every overlay starts hidden and is shown by its `GlobalStates`
        // flag.
        Layer::Overlay => builder
            .always_on_top(stays_on_top(surface.label))
            .visible(born_visible),
    };

    // Tauri takes every drag over its windows to report dropped files,
    // and on Windows that ends a page's own drag the moment it starts: the
    // pages' own drags could not move anything.
    // Nor did a file dropped on the shelf ever reach it that way; the
    // shelf has its own way in, below.
    builder = builder.disable_drag_drop_handler();

    let window = builder.build()?;
    apply_layer(app, &window, surface.layer, &config);
    // Born hidden, it is told so: the webview itself starts out visible.
    if !born_visible {
        set_webview_visible(&window, false);
    }
    #[cfg(windows)]
    if surface.label == SHELF.label {
        let app = app.clone();
        crate::platform::dragout::accept_drops(&window, move |_, paths| {
            use tauri::Emitter;
            let _ = app.emit_to(SHELF.label, crate::commands::event::SHELF_DROPPED, paths);
        });
    }
    // A flag set before this window existed — `bw settings open` sent while
    // the shell was still starting, say — found nothing to show and was
    // dropped, and nothing looks at it again: the request vanished, and
    // left the flag saying the surface was open when it was not.
    if flagged_open(&app.state::<AppState>().states(), surface.label) {
        set_visible(app, &label, true)?;
    }
    Ok(())
}

/// Where a surface sits on `screen`, in its logical pixels.
fn geometry(surface: &Surface, config: &bw_core::Config, screen: &Screen) -> (f64, f64, f64, f64) {
    match surface.layer {
        Layer::Background | Layer::Chrome => (0.0, 0.0, screen.size.0, screen.size.1),
        Layer::Overlay => overlay_geometry(surface, config, screen),
    }
}

/// Puts a surface's window where [`geometry`] says on `screen`.
fn put(
    window: &tauri::WebviewWindow,
    surface: &Surface,
    config: &bw_core::Config,
    screen: &Screen,
) {
    let (x, y, width, height) = geometry(surface, config, screen);
    let physical = |value: f64| (value * screen.scale).round() as i32;
    // Moved, then sized: moved onto a monitor at another scale, Windows
    // resizes the window for that scale, and the size given after it is the
    // one that stands.
    let _ = window.set_position(tauri::PhysicalPosition::new(
        screen.origin.0 + physical(x),
        screen.origin.1 + physical(y),
    ));
    let _ = window.set_size(tauri::PhysicalSize::new(
        physical(width).max(1) as u32,
        physical(height).max(1) as u32,
    ));
}

/// Whether a surface is kept above every other program's windows. Not the
/// ones that are windows to work in, which another program's window or menu
/// may come over: Win+X's menu opened behind the settings (#231).
fn stays_on_top(label: &str) -> bool {
    ![SETTINGS.label, TIPS.label, WIZARD.label].contains(&label)
}

/// Whether an overlay opens on the monitor the pointer is on. The widget
/// overlay does not: its widgets are placed in the primary monitor's pixels,
/// and its window region is cut to them there.
fn follows_the_pointer(surface: &Surface) -> bool {
    surface.layer == Layer::Overlay
        && surface.label != OVERLAY.label
        && surface.label != OVERLAY_PINNED.label
}

/// Takes a native bar's window each time `bw-native` places it, on that
/// window's own thread: hears Explorer coming back and the monitors changing
/// through it, and reserves its edge unless it hides itself. Answers the
/// rectangle the bar is to occupy.
pub fn adopt_native_bar(
    app: &AppHandle,
    hwnd: isize,
    monitor: bw_native::Rect,
    vertical: bool,
    thickness: i32,
) -> Option<bw_native::Rect> {
    // Subclassed from here because a window can only be subclassed from the
    // thread that owns it, which is this one. Again for a window already
    // watched changes nothing.
    #[cfg(windows)]
    {
        let app = app.clone();
        unsafe {
            crate::platform::win::watch_edge_window(
                windows::Win32::Foundation::HWND(hwnd as _),
                move || desktop_changed(&app),
            );
        }
    }
    let config = app.state::<AppState>().config();
    if !config.bar.enable || !config.bar.reserve_space || config.bar.auto_hide {
        return None;
    }
    reserve_native_bar(app, hwnd, monitor, vertical, thickness)
}

/// Where the config puts the bars: on the primary monitor, or on every one.
pub fn bar_layout(config: &bw_core::Config) -> bw_native::BarLayout {
    #[cfg(windows)]
    let monitors = crate::platform::win::monitors()
        .into_iter()
        .filter(|monitor| config.bar.per_monitor || monitor.primary)
        .map(|monitor| (monitor.x, monitor.y, monitor.width, monitor.height))
        .collect();
    #[cfg(not(windows))]
    let monitors = vec![(0, 0, 1920, 1080)];
    bw_native::BarLayout {
        monitors,
        vertical: config.bar.vertical,
        auto_hide: config.bar.auto_hide,
        hover: config.bar.hover_region_height as i32,
    }
}

/// Puts everything back after Explorer has restarted, or the monitors have
/// changed: the taskbar, the bar's edge, the overlays, the wallpaper surface.
///
/// On a thread of its own, not the one that heard it and not queued back onto
/// it. Explorer sends `TaskbarCreated` to every top-level window in turn and
/// waits on each; reserving the bar's edge asks Explorer for it, and asked from
/// that thread, it waited on an Explorer still waiting on the shell. Neither
/// ever answered: the shell hung for good on every restart of Explorer.
///
/// A monitor change arrives as a burst, one message per step Windows takes, so
/// a pass that is already running is told to go round once more rather than
/// ignoring the news. A change that lands after the last look but before the
/// lock is let go of finds it held and leaves; the pass looks once more after
/// letting go, and takes that change up itself.
pub fn desktop_changed(app: &AppHandle) {
    static PLACING: Mutex<()> = Mutex::new(());
    static AGAIN: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    use std::sync::atomic::Ordering::SeqCst;
    AGAIN.store(true, SeqCst);
    let app = app.clone();
    std::thread::spawn(move || loop {
        let Some(placing) = PLACING.try_lock() else {
            return;
        };
        // Half a second for the burst to settle into one pass.
        std::thread::sleep(std::time::Duration::from_millis(500));
        while AGAIN.swap(false, SeqCst) {
            // The taskbar first: a new Explorer may have forgotten it was
            // hiding itself, and the bar asks for what is left.
            forget_dead_background(&app);
            crate::services::integration::reassert_taskbar(&app);
            place_bars(&app);
            crate::services::integration::recompute_work_areas(&app);
            place_overlays(&app);
            crate::services::chrome::emit(&app);
            restore_background(&app);
        }
        drop(placing);
        if !AGAIN.load(SeqCst) {
            return;
        }
    });
}

/// Puts the bars where the config now says: shown or not, on which monitors,
/// along which edge, and with their edges reserved or not. The edges are given
/// back first and taken again as `bw-native` places each bar, so a bar that
/// was switched off, or a work area Explorer forgot, is put right.
pub fn place_bars(app: &AppHandle) {
    let config = app.state::<AppState>().config();
    release_reservations(app);
    bw_native::bar_shown(config.bar.enable);
    bw_native::bars(bar_layout(&config));
}

/// The wallpaper surface's window label: `background`, until Explorer takes
/// that window down with it. Tauri never hears it go — the window is
/// destroyed along with Explorer's, from Explorer's side — so it goes on
/// holding the label, and a window by the same name cannot be made again.
/// Each one made after that is `background-1`, `background-2` and so on.
static BACKGROUND_LABEL: Mutex<Option<String>> = Mutex::new(None);

pub fn background_label() -> String {
    BACKGROUND_LABEL
        .lock()
        .clone()
        .unwrap_or_else(|| BACKGROUND.label.to_owned())
}

/// Makes the wallpaper surface again after Explorer has restarted.
///
/// It lives inside Explorer's desktop window, so a restart took it down with
/// that window: the desktop was left showing Windows' own copy of the picture,
/// with no widgets, until the shell was restarted too.
///
/// Called off the main thread, and waits there for the new desktop first:
/// the taskbar can announce itself before the desktop exists, and a
/// background given no desktop to go into would be an ordinary full-screen
/// window, lying over everything that sits on the desktop.
pub fn restore_background(app: &AppHandle) {
    #[cfg(windows)]
    {
        // Ten seconds, and no longer: this runs inside the pass that puts
        // everything back, and a desktop slower than that is not waited on
        // there. It is owed instead, and `restore_owed_background` makes it
        // once the desktop turns up.
        for _ in 0..40 {
            if crate::platform::win::desktop_exists() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(250));
        }
        if !crate::platform::win::desktop_exists() {
            BACKGROUND_OWED.store(true, std::sync::atomic::Ordering::Relaxed);
            return;
        }
        BACKGROUND_OWED.store(false, std::sync::atomic::Ordering::Relaxed);
        // A moment more, so the desktop has its icon view.
        std::thread::sleep(std::time::Duration::from_secs(1));
    }

    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        // Still here: it was not inside the Explorer that went. Put it under
        // the icons of the one that came. A dead one was let go of already.
        if let Some(window) = handle.get_webview_window(&background_label()) {
            let config = handle.state::<AppState>().config();
            apply_layer(&handle, &window, Layer::Background, &config);
            return;
        }
        if let Err(error) = ensure(&handle, &BACKGROUND) {
            tracing::warn!(%error, "could not make the wallpaper surface again");
        }
    });
}

/// Whether a wallpaper surface is waiting for a desktop to go into.
#[cfg(windows)]
static BACKGROUND_OWED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Makes the wallpaper surface [`restore_background`] gave up waiting for,
/// once there is a desktop for it. Looked at every second.
pub fn restore_owed_background(app: &AppHandle) {
    #[cfg(windows)]
    if BACKGROUND_OWED.load(std::sync::atomic::Ordering::Relaxed)
        && crate::platform::win::desktop_exists()
    {
        restore_background(app);
    }
    #[cfg(not(windows))]
    let _ = app;
}

/// Lets go of a wallpaper surface that went down with Explorer.
///
/// Until it is let go of, every call Tauri makes on its webview fails, and
/// each failure is a line in the log. Checked every second, and again when
/// Explorer announces itself; a live surface is left alone.
pub fn forget_dead_background(app: &AppHandle) {
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        let Some(window) = handle.get_webview_window(&background_label()) else {
            return;
        };
        #[cfg(windows)]
        let alive = window.hwnd().is_ok_and(|hwnd| unsafe {
            windows::Win32::UI::WindowsAndMessaging::IsWindow(windows::Win32::Foundation::HWND(
                hwnd.0,
            ))
            .as_bool()
        });
        #[cfg(not(windows))]
        let alive = true;
        if alive {
            return;
        }
        // Gone underneath Tauri. Letting go of it drops its webview; the
        // label stays taken all the same, so the next one needs its own.
        let _ = window.destroy();
        static REMADE: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let made = REMADE.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
        *BACKGROUND_LABEL.lock() = Some(format!("{}-{made}", BACKGROUND.label));
    });
}

/// Puts every overlay where the config now says. Most of them keep clear of
/// the bar, so moving the bar moves them too.
pub fn place_overlays(app: &AppHandle) {
    let config = app.state::<AppState>().config();
    let primary = Screen::primary(app, &config);
    for surface in ALL.iter().filter(|surface| surface.layer == Layer::Overlay) {
        let Some(window) = app.get_webview_window(surface.label) else {
            continue;
        };
        // Left on the monitor it was opened on, unless that one has gone.
        if follows_the_pointer(surface) {
            put(
                &window,
                surface,
                &config,
                &Screen::of_window(app, &window, &config),
            );
        } else {
            put(&window, surface, &config, &primary);
        }
    }

    // The decorations cover the primary monitor, which a monitor change can
    // resize or replace.
    let screen = primary.size;
    for surface in ALL.iter().filter(|surface| surface.layer == Layer::Chrome) {
        if let Some(window) = app.get_webview_window(surface.label) {
            // A maximized window keeps its maximized place whatever it is told.
            #[cfg(windows)]
            if let Ok(handle) = window.hwnd() {
                unsafe {
                    crate::platform::win::unmaximize(windows::Win32::Foundation::HWND(handle.0))
                };
            }
            let _ = window.set_position(tauri::LogicalPosition::new(0.0, 0.0));
            let _ = window.set_size(tauri::LogicalSize::new(screen.0, screen.1));
        }
    }
}

/// Where an overlay sits.
///
/// Most overlays are centred; the sidebars keep clear of the bar rather than
/// sliding under it.
fn overlay_geometry(
    surface: &Surface,
    config: &bw_core::Config,
    screen: &Screen,
) -> (f64, f64, f64, f64) {
    let has_bar = screen.has_bar;
    let (inset_left, inset_top, inset_right, inset_bottom) = screen.work_inset;
    let screen = screen.size;
    let (fraction_w, fraction_h) = surface.size.unwrap_or((0.5, 0.5));
    let (width, height) = (screen.0 * fraction_w, screen.1 * fraction_h);
    // Along the top, or down the left side when it is vertical.
    let (bar_top, bar_left) = match (has_bar, config.bar.vertical) {
        (false, _) => (0.0, 0.0),
        (true, false) => (f64::from(bw_native::HEIGHT), 0.0),
        (true, true) => (0.0, f64::from(bw_native::WIDTH)),
    };
    // Inside the work area too: a Windows taskbar left showing covered the
    // foot of the sidebars, which is where the calendar is (#230). The bar's
    // own reservation is part of the work area already, hence the larger of
    // the two rather than both.
    let (top, left) = (bar_top.max(inset_top), bar_left.max(inset_left));
    let right = screen.0 - inset_right;
    let margin = 8.0;
    let tall = (screen.1 - inset_bottom - top - margin * 2.0).max(1.0);

    if surface.label == SIDEBAR_LEFT.label {
        return (
            left + margin,
            top + margin,
            screen.0 * config.sidebar.left.width,
            tall,
        );
    }

    if surface.label == SHELF.label {
        // Against whichever edge was asked for, and clear of the bar rather
        // than sliding under it — the same reasoning as the sidebars.
        let width = screen.0 * config.shelf.width;
        let x = if config.shelf.edge == "left" {
            left + margin
        } else {
            right - width - margin
        };
        return (x, top + margin, width, tall);
    }

    if surface.label == SIDEBAR_RIGHT.label {
        // Pinned to the right edge and as tall as the work area allows, kept
        // clear of the bar rather than sliding under it.
        let width = screen.0 * config.sidebar.width;
        return (right - width - margin, top + margin, width, tall);
    }

    (
        (screen.0 - width) / 2.0,
        (screen.1 - height) / 2.0,
        width,
        height,
    )
}

#[cfg(windows)]
fn apply_layer(
    app: &AppHandle,
    window: &tauri::WebviewWindow,
    layer: Layer,
    config: &bw_core::Config,
) {
    use crate::platform::win::{self, Layer as WinLayer};
    use windows::Win32::Foundation::HWND;

    let Ok(handle) = window.hwnd() else {
        tracing::warn!(
            surface = window.label(),
            "a surface has no window handle yet"
        );
        return;
    };
    let hwnd = HWND(handle.0);

    let target = match layer {
        Layer::Background => WinLayer::Wallpaper,
        _ if !stays_on_top(window.label()) => WinLayer::Normal,
        Layer::Overlay | Layer::Chrome => WinLayer::Overlay,
    };

    // Everything that goes through Tao comes first, because Tao owns these
    // windows' styles: each of its calls writes `GWL_EXSTYLE` whole, from its
    // own flags, and erases anything set behind its back. Doing it the other
    // way round cost `screenChrome` its `WS_EX_TOOLWINDOW`, which left it an
    // ordinary application window: one Alt-Tab lists.
    //
    // The order is only half of it. Tao writes the styles again on every show
    // and hide, long after this ran, so `WS_EX_TOOLWINDOW` is held by the
    // subclass `set_layer` installs rather than by getting the order right.
    // `WS_EX_NOACTIVATE` is Tao's own flag, set by the builder in `ensure`.

    // The decorations are drawn over everything and pressed by nobody.
    //
    // Tauri's own call rather than `WS_EX_TRANSPARENT` by hand: on Windows a
    // transparent window is only see-through to the pointer when it is layered
    // as well, and setting one without the other leaves a window that looks
    // click-through in a list of styles and swallows every click underneath it.
    // `screenChrome` covers the screen, so that was every click on the desktop.
    if layer == Layer::Chrome {
        if let Err(error) = window.set_ignore_cursor_events(true) {
            tracing::warn!(%error, surface = window.label(), "could not let the pointer through a surface");
        }
    }

    unsafe {
        if let Err(error) = win::set_layer(hwnd, target) {
            tracing::warn!(%error, "could not place the surface on its layer");
        }
    }

    // The desktop blurred behind the panel, as a Mac's are. Two limits pick
    // the surfaces. Windows blurs the whole window, so the window has to be
    // exactly the panel, or the blur shows in the empty space around it. And
    // it draws the blur only while the window is active, falling back to a
    // flat colour otherwise, so a surface that never takes focus — the bar,
    // the dock, the readout, the toasts — would only ever show the flat
    // colour. That leaves the sidebars and the shelf. `auto` is Acrylic, the
    // blur of what is behind; Mica only tints by the wallpaper.
    // ponytail: the full-screen surfaces that do take focus (search, menus,
    // settings) need their window cut to the panel's shape first.
    let fits = [SIDEBAR_RIGHT.label, SIDEBAR_LEFT.label, SHELF.label].contains(&window.label());
    if fits && config.windows.backdrop != "none" {
        let backdrop = if config.windows.backdrop == "mica" {
            win::Backdrop::Mica
        } else {
            win::Backdrop::Acrylic
        };
        unsafe {
            win::set_backdrop(hwnd, backdrop, config.appearance.palette.mode != "light");
        }
    }
    let _ = app;
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
        if let (true, Some(surface)) = (visible, on_demand(label)) {
            // On the main thread, and not inside whatever asked: that is often
            // a command from another page, and on Windows building a window
            // from inside one deadlocks. `ensure` shows it once it exists, if
            // it is still wanted then.
            let app = app.clone();
            later(&app.clone(), move || {
                if flagged_open(&app.state::<AppState>().states(), surface.label) {
                    if let Err(error) = ensure(&app, surface) {
                        tracing::error!(%error, surface = surface.label, "could not create a surface");
                    }
                }
            });
        }
        return Ok(());
    };
    if visible {
        // Only as it opens: every change of any flag comes through here for
        // every surface still open, and an open sidebar would follow the
        // pointer to the other monitor at the next one.
        let opening = !window.is_visible().unwrap_or(false);
        if let Some(surface) = ALL.iter().find(|surface| surface.label == label) {
            if opening && follows_the_pointer(surface) {
                let config = app.state::<AppState>().config();
                put(
                    &window,
                    surface,
                    &config,
                    &Screen::under_pointer(app, &config),
                );
            }
        }
        // An overlay only takes focus when the user opened it deliberately.
        // The readout and the toasts never do — taking focus from whatever the
        // user is typing into would be worse than the information is worth.
        if takes_focus(label) {
            // Opened on purpose, so it may be clicked into as well: kept
            // unactivatable, a click on it left the keyboard where it was, and
            // Escape — the only way out of a sidebar — never reached it.
            window.set_focusable(true)?;
            set_webview_visible(&window, true);
            window.show()?;
            window.set_focus()?;
        } else {
            show_without_taking_the_foreground(&window)?;
        }
    } else if on_demand(label).is_some() {
        // Later, for the same reason it is made later — the request to close
        // it usually comes from its own page — and only if it was not opened
        // again in the meantime.
        let app = app.clone();
        let label = label.to_owned();
        later(&app.clone(), move || {
            if !flagged_open(&app.state::<AppState>().states(), &label) {
                if let Some(window) = app.get_webview_window(&label) {
                    let _ = window.destroy();
                }
            }
        });
    } else {
        hide(&window)?;
        window.set_focusable(false)?;
    }
    Ok(())
}

/// Hides a surface, and tells its page so.
pub fn hide(window: &tauri::WebviewWindow) -> tauri::Result<()> {
    set_webview_visible(window, false);
    window.hide()
}

/// Tells WebView2 whether the page can be seen, and how much memory it may
/// keep.
///
/// Hiding the window does not: WebView2 only knows what its host tells it, and
/// a hidden surface went on as if on screen — its clock ticking every second,
/// its animations drawing, its memory held — for as long as the shell ran.
/// Told it is not visible, it throttles the page's timers, stops drawing it,
/// and at the low memory target gives back what it can.
pub fn set_webview_visible(window: &tauri::WebviewWindow, visible: bool) {
    #[cfg(windows)]
    {
        use webview2_com::Microsoft::Web::WebView2::Win32::{
            ICoreWebView2_19, COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW,
            COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_NORMAL,
        };
        use webview2_windows_core::Interface;

        let _ = window.with_webview(move |webview| unsafe {
            let controller = webview.controller();
            if let Err(error) = controller.SetIsVisible(visible) {
                tracing::debug!(%error, "could not tell a webview whether it is visible");
            }
            let level = if visible {
                COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_NORMAL
            } else {
                COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW
            };
            if let Ok(core) = controller
                .CoreWebView2()
                .and_then(|core| core.cast::<ICoreWebView2_19>())
            {
                let _ = core.SetMemoryUsageTargetLevel(level);
            }
        });
    }
    #[cfg(not(windows))]
    let _ = (window, visible);
}

/// Shows a window and leaves the foreground where it was.
///
/// `ensure` builds every surface with `focused(false)`, which is what makes Tao
/// show it with `SW_SHOWNOACTIVATE` — every time: Tao clears its marker on a
/// copy of the flags, never on the ones it keeps. Left at the default, Tao
/// shows with `SW_SHOW`, and on any change of flags while the window is
/// visible, not only on a show. `focusable(false)` adds `WS_EX_NOACTIVATE`,
/// so a click does not activate it either; only a program naming it, through
/// `SetForegroundWindow` or `SetActiveWindow`, does.
///
/// The surfaces shown through here are the ones that most need it: off screen,
/// or a strip a few pixels tall, or transparent, so a user who lost the
/// foreground to one can neither see it nor click it, and has nothing to click
/// to take the foreground back. It reads as the whole desktop
/// refusing the mouse rather than as anything to do with a notification.
pub fn show_without_taking_the_foreground(window: &tauri::WebviewWindow) -> tauri::Result<()> {
    set_webview_visible(window, true);
    window.show()
}

/// Whether showing this surface should also focus it.
fn takes_focus(label: &str) -> bool {
    // Only a surface the user opened on purpose may take the focus off what
    // they were doing, so this asks which surfaces those are rather than
    // listing the ones that must not. Naming the exceptions was what let the
    // decorations through: the screen chrome is opened by nobody, are shown again every time the config is applied, and were on
    // the permitted side of a list of three.
    //
    // Taking the focus is not a small thing here. `set_focus` puts the window
    // in the foreground, and these surfaces are a masked strip, a window off
    // the side of the screen, or transparent — nothing the user can see or
    // click to take it back. Every click they make afterwards lands on a
    // window that never becomes active, which reads as the mouse having died
    // rather than as anything to do with a shell surface.
    let Some(surface) = ALL.iter().find(|surface| surface.label == label) else {
        return false;
    };

    // The background and the decorations are furniture; nobody opens them.
    // The overlay's passive half is never opened at all.
    surface.layer == Layer::Overlay && label != OVERLAY_PINNED.label
}

#[cfg(test)]
mod tests {
    /// Every surface is made at startup or on demand, and only once.
    #[test]
    fn every_surface_has_one_time_to_be_made() {
        let mut made: Vec<&str> = super::FIRST
            .iter()
            .chain(super::AFTER)
            .chain(super::ON_DEMAND)
            .chain([&super::SCREEN_CHROME])
            .map(|surface| surface.label)
            .collect();
        let mut all: Vec<&str> = super::ALL.iter().map(|surface| surface.label).collect();
        made.sort_unstable();
        all.sort_unstable();
        assert_eq!(made, all);
    }

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

    /// The toast surface covers a quarter of the screen and is not
    /// click-through, so the desktop is only usable while it is parked.
    /// Getting this wrong takes the foreground and does not give it back: the
    /// surfaces that must not take it are ones the user cannot see or click,
    /// so nothing hands it on, and every click afterwards lands on a window
    /// that never becomes active. It reads as a dead mouse, not as a shell
    /// surface misbehaving, which is what made it so hard to find.
    #[test]
    fn only_the_surfaces_a_user_opens_may_take_the_focus() {
        for surface in [
            SETTINGS,
            OVERVIEW,
            SESSION,
            SHELF,
            WIZARD,
            TIPS,
            WALLPAPER_SELECTOR,
            SIDEBAR_LEFT,
            SIDEBAR_RIGHT,
            DESKTOP_MENU,
            REGION_SELECT,
            OVERLAY,
        ] {
            assert!(
                takes_focus(surface.label),
                "{} is opened on purpose",
                surface.label
            );
        }

        for surface in [BACKGROUND, SCREEN_CHROME, OVERLAY_PINNED] {
            assert!(
                !takes_focus(surface.label),
                "{} is never opened by anyone",
                surface.label
            );
        }
    }

    #[test]
    fn a_surface_asked_for_before_it_existed_is_opened_when_it_does() {
        let mut states = crate::state::GlobalStates::default();
        assert!(!flagged_open(&states, SETTINGS.label));

        states.settings_open = true;
        assert!(flagged_open(&states, SETTINGS.label));
        assert!(!flagged_open(&states, WIZARD.label));
        // The furniture has no flag, so nothing may open it by accident.
        assert!(!flagged_open(&states, SCREEN_CHROME.label));
    }

    #[test]
    fn the_background_covers_the_screen_and_overlays_do_not() {
        assert!(BACKGROUND.size.is_none());
        let (w, h) = WALLPAPER_SELECTOR.size.expect("an overlay needs a size");
        assert!((0.0..=1.0).contains(&w) && (0.0..=1.0).contains(&h));
    }

    #[test]
    fn only_the_windows_to_work_in_let_others_over_them() {
        for surface in [SETTINGS, TIPS, WIZARD] {
            assert!(!stays_on_top(surface.label), "{}", surface.label);
        }
        for surface in [SIDEBAR_RIGHT, OVERVIEW, SCREEN_CHROME] {
            assert!(stays_on_top(surface.label), "{}", surface.label);
        }
    }

    /// A Windows taskbar left showing along the bottom: the sidebars stop
    /// above it, where the calendar at their foot can be seen (#230).
    #[test]
    fn the_sidebars_stop_short_of_a_taskbar() {
        let config = bw_core::Config::default();
        let screen = Screen {
            origin: (0, 0),
            size: (1920.0, 1080.0),
            scale: 1.0,
            has_bar: true,
            work_inset: (0.0, 0.0, 0.0, 48.0),
        };
        for surface in [SIDEBAR_LEFT, SIDEBAR_RIGHT, SHELF] {
            let (_, y, _, height) = overlay_geometry(&surface, &config, &screen);
            assert_eq!(y + height, 1080.0 - 48.0 - 8.0, "{}", surface.label);
        }

        // With nothing reserved, the bar alone decides the top.
        let bare = Screen {
            work_inset: (0.0, 0.0, 0.0, 0.0),
            ..screen
        };
        let (_, y, _, _) = overlay_geometry(&SIDEBAR_RIGHT, &config, &bare);
        assert_eq!(y, f64::from(bw_native::HEIGHT) + 8.0);
    }
}
