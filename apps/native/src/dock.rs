//! The dock, as the webview's `Dock.tsx` draws it and behaves.
//!
//! Its icons come from the shell's dock reading; what is pressed goes back as
//! an [`Action`]. Hiding is a window that is not there: while the dock is
//! hidden it has no window on screen at all, and the pointer reaching the
//! bottom edge under it — looked for ten times a second — brings it back.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use serde_json::Value;
use slint::{ComponentHandle, ModelRc, VecModel};

use crate::{icon, round_corners, screen_size, settle, window_of, Action, Dock, DockItem};

/// What the dock needs from the config.
#[derive(Debug, Clone, PartialEq)]
pub struct DockSettings {
    pub enable: bool,
    pub icon_size: u32,
    pub show_media: bool,
    pub show_pin_button: bool,
    pub auto_hide: bool,
    /// Physical pixels from the bottom edge that bring a hidden dock back.
    pub hover_region_height: u32,
    pub pinned_on_startup: bool,
}

#[derive(Default)]
struct State {
    window: Option<Dock>,
    settings: Option<DockSettings>,
    /// The applications as the shell sent them, pinned first, which a press
    /// hands back.
    pinned: Vec<Value>,
    running: Vec<Value>,
    media_title: String,
    media_playing: bool,
    /// Kept up by its own button, whatever `auto_hide` says.
    kept: bool,
    hovered: bool,
    revealed: bool,
    settled: bool,
    act: Option<Rc<dyn Fn(Action)>>,
    /// Looks for the pointer at the bottom edge while the dock is hidden, and
    /// puts it away a moment after the pointer leaves.
    watch: Option<slint::Timer>,
    leave: Option<slint::Timer>,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}

/// Recolours the dock, as the theme goes light or dark.
pub(crate) fn repaint(light: bool) {
    STATE.with(|state| {
        if let Some(window) = &state.borrow().window {
            window.global::<crate::Palette>().set_light(light);
        }
    });
}

pub(crate) fn set_act(act: Rc<dyn Fn(Action)>) {
    STATE.with(|state| state.borrow_mut().act = Some(act));
}

/// The settings, made or changed. Native thread only.
pub(crate) fn configure(settings: DockSettings) {
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        if state.settings.is_none() {
            state.kept = settings.pinned_on_startup;
        }
        state.settings = Some(settings.clone());
    });
    if !settings.enable {
        STATE.with(|state| {
            let mut state = state.borrow_mut();
            if let Some(window) = &state.window {
                let _ = window.hide();
            }
            state.watch = None;
            state.revealed = false;
        });
        return;
    }
    if STATE.with(|state| state.borrow().window.is_none()) {
        match make_window() {
            Ok(window) => STATE.with(|state| state.borrow_mut().window = Some(window)),
            Err(error) => {
                tracing::warn!(%error, "could not make the dock");
                return;
            }
        }
    }
    redraw();
    let watch = slint::Timer::default();
    watch.start(
        slint::TimerMode::Repeated,
        Duration::from_millis(100),
        look_for_pointer,
    );
    STATE.with(|state| state.borrow_mut().watch = Some(watch));
    reveal(!wants_hiding());
}

/// A reading the shell sent: the dock's applications, or the playing track.
pub(crate) fn apply(event: &str, value: &Value) {
    match event {
        "bw://dock" => {
            let (pinned, running): (Vec<Value>, Vec<Value>) = value
                .as_array()
                .into_iter()
                .flatten()
                .cloned()
                .partition(|app| app["pinned"].as_bool().unwrap_or(false));
            STATE.with(|state| {
                let mut state = state.borrow_mut();
                state.pinned = pinned;
                state.running = running;
            });
        }
        "bw://media" => STATE.with(|state| {
            let mut state = state.borrow_mut();
            state.media_title = value["title"].as_str().unwrap_or_default().to_owned();
            state.media_playing = value["playing"].as_bool().unwrap_or(false);
        }),
        _ => return,
    }
    redraw();
}

fn item(app: &Value) -> DockItem {
    let icon = app["icon"]
        .as_str()
        .filter(|path| !path.is_empty())
        .and_then(|path| slint::Image::load_from_path(path.as_ref()).ok())
        .unwrap_or_default();
    DockItem {
        icon,
        name: app["name"].as_str().unwrap_or_default().into(),
        running: app["windows"]
            .as_array()
            .is_some_and(|windows| !windows.is_empty()),
        active: app["active"].as_bool().unwrap_or(false),
    }
}

/// Puts what is known into the window, and the window where it belongs.
fn redraw() {
    STATE.with(|state| {
        let state = state.borrow();
        let (Some(window), Some(settings)) = (&state.window, &state.settings) else {
            return;
        };
        let pinned: Vec<DockItem> = state.pinned.iter().map(item).collect();
        let running: Vec<DockItem> = state.running.iter().map(item).collect();
        window.set_pinned_apps(ModelRc::new(VecModel::from(pinned)));
        window.set_running_apps(ModelRc::new(VecModel::from(running)));
        window.set_icon_size(settings.icon_size as f32);
        window.set_show_media(settings.show_media);
        window.set_media_title(state.media_title.as_str().into());
        let glyph = if state.media_playing {
            "pause"
        } else {
            "play_arrow"
        };
        window.set_media_glyph(icon(glyph, 20.0, false));
        window.set_show_keep(settings.show_pin_button);
        window.set_kept(state.kept);
        window.set_keep_glyph(icon(
            if state.kept { "keep" } else { "keep_off" },
            18.0,
            state.kept,
        ));
    });
    // Its size follows what is in it, which the layout settles on the next turn.
    slint::Timer::single_shot(Duration::ZERO, place);
}

/// The dock's size in physical pixels: what its content wants, not what its
/// window last was — a hidden window keeps the size it had until it is shown.
fn wanted(window: &Dock) -> (i32, i32) {
    let scale = window.window().scale_factor();
    (
        (window.get_wanted_width() * scale).ceil() as i32,
        (window.get_wanted_height() * scale).ceil() as i32,
    )
}

/// Centred along the bottom, 8 pixels up.
fn place() {
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        let Some(window) = &state.window else { return };
        let (screen_width, screen_height) = screen_size();
        let scale = window.window().scale_factor();
        let (width, height) = wanted(window);
        let x = (screen_width - width) / 2;
        let y = screen_height - height - (8.0 * scale) as i32;
        window.window().set_size(slint::PhysicalSize::new(
            width.max(1) as u32,
            height.max(1) as u32,
        ));
        window
            .window()
            .set_position(slint::PhysicalPosition::new(x, y));
        if !state.settled {
            if let Some(hwnd) = window_of(window.window()) {
                settle(hwnd, None);
                round_corners(hwnd);
                state.settled = true;
            } else {
                slint::Timer::single_shot(Duration::from_millis(50), place);
            }
        }
    });
}

/// Whether the dock should be out of the way now.
fn wants_hiding() -> bool {
    STATE.with(|state| {
        let state = state.borrow();
        let window = state.window.as_ref();
        let dragging = window.is_some_and(|window| window.get_dragged() >= 0);
        state
            .settings
            .as_ref()
            .is_some_and(|settings| settings.auto_hide)
            && !state.kept
            && !state.hovered
            && !dragging
    })
}

fn reveal(shown: bool) {
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        state.revealed = shown;
        if let Some(window) = &state.window {
            let _ = if shown { window.show() } else { window.hide() };
        }
    });
    if shown {
        place();
    }
}

/// While hidden: the pointer on the bottom edge, under where the dock would
/// be or within 24 pixels of either end — the shadow the webview dock's
/// window took in — brings it back.
fn look_for_pointer() {
    let (hidden, band, edge) = STATE.with(|state| {
        let state = state.borrow();
        let Some(window) = &state.window else {
            return (false, (0, 0), 0);
        };
        let (width, _) = wanted(window);
        let (screen_width, screen_height) = screen_size();
        let x = (screen_width - width) / 2;
        let reach = (24.0 * window.window().scale_factor()) as i32;
        let strip = state
            .settings
            .as_ref()
            .map_or(3, |settings| settings.hover_region_height.max(1) as i32);
        (
            !state.revealed,
            (x - reach, x + width + reach),
            screen_height - strip,
        )
    });
    if !hidden {
        return;
    }
    let (x, y) = cursor();
    if y >= edge && x >= band.0 && x < band.1 {
        reveal(true);
    }
}

pub(crate) fn cursor() -> (i32, i32) {
    #[cfg(windows)]
    unsafe {
        use windows::Win32::Foundation::POINT;
        use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;
        let mut point = POINT::default();
        let _ = GetCursorPos(&mut point);
        (point.x, point.y)
    }
    #[cfg(not(windows))]
    (0, 0)
}

fn make_window() -> Result<Dock, slint::PlatformError> {
    let window = Dock::new()?;
    window
        .global::<crate::Palette>()
        .set_light(crate::is_light());
    window.set_fallback(icon("apps", 24.0, false));

    window.on_hovered(|inside| {
        STATE.with(|state| state.borrow_mut().hovered = inside);
        if inside {
            STATE.with(|state| state.borrow_mut().leave = None);
            return;
        }
        // A moment's grace, as the pointer crosses a gap or the edge.
        let leave = slint::Timer::default();
        leave.start(
            slint::TimerMode::SingleShot,
            Duration::from_millis(400),
            || {
                if wants_hiding() {
                    reveal(false);
                }
            },
        );
        STATE.with(|state| state.borrow_mut().leave = Some(leave));
    });
    window.on_activate(|pinned, index| {
        if let Some(app) = app_at(pinned, index) {
            act(Action::DockActivate(app));
        }
    });
    window.on_pin(|pinned, index| {
        if let Some(app) = app_at(pinned, index) {
            let executable = app["executable"].as_str().unwrap_or_default().to_owned();
            act(Action::DockPin(executable, !pinned));
        }
    });
    window.on_move(|from, to| {
        let (Some(moved), Some(target)) = (app_at(true, from), app_at(true, to)) else {
            return;
        };
        let path = |app: &Value| app["executable"].as_str().unwrap_or_default().to_owned();
        if from != to {
            act(Action::DockMove(path(&moved), path(&target)));
        }
    });
    window.on_media(|| act(Action::Media("playPause".to_owned())));
    window.on_keep(|| {
        STATE.with(|state| {
            let mut state = state.borrow_mut();
            state.kept = !state.kept;
        });
        redraw();
    });
    Ok(window)
}

fn app_at(pinned: bool, index: i32) -> Option<Value> {
    STATE.with(|state| {
        let state = state.borrow();
        let list = if pinned {
            &state.pinned
        } else {
            &state.running
        };
        list.get(usize::try_from(index).ok()?).cloned()
    })
}

fn act(action: Action) {
    if let Some(act) = STATE.with(|state| state.borrow().act.clone()) {
        act(action);
    }
}
