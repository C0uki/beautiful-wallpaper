//! The always-on surfaces, drawn natively with Slint instead of in webviews:
//! the bar, the volume readout, the notification banners and the dock.
//!
//! Without Chromium behind it the bar is on screen in about a second and costs
//! about 10 MB, where the webview bar needs WebView2's browser and GPU
//! processes — some 300 MB on their own — before it can draw at all.
//!
//! They run on a thread of their own with its own event loop, beside Tauri's:
//! on Windows Slint's winit backend allows that, and its windows never touch the
//! shell's. They read nothing themselves but the clock. The shell hands them
//! the readings it sends the webview surfaces, through [`update`], asks for the
//! readout through [`osd`], the banners through [`toasts`], the dock's
//! settings through [`dock`], the clock's through [`bar`] and whether the theme
//! is light through [`light`], and hears what was pressed through the callback
//! given to [`spawn`].

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use chrono::{Datelike, Timelike};
use serde::Deserialize;
use serde_json::Value;
use slint::{ComponentHandle, Model, ModelRc, VecModel};

slint::include_modules!();

mod dock;
mod toasts;
pub use dock::DockSettings;
pub use toasts::ToastSettings;
include!(concat!(env!("OUT_DIR"), "/icons.rs"));

/// The bar's height at 100%, in logical pixels.
pub const HEIGHT: f32 = 30.0;

/// Whether the thread was started, so [`update`] and [`osd`] cost nothing when
/// it was not.
static RUNNING: AtomicBool = AtomicBool::new(false);

/// The last notifications and settings sent, kept for the thread to pick up
/// when it is ready: the shell posts its first notices while it is still
/// starting, before there is a loop to send them to.
static PENDING_TOASTS: std::sync::Mutex<Option<(Value, ToastSettings)>> =
    std::sync::Mutex::new(None);
/// Whether the bar is wanted on screen: `bar.enable`. Kept rather than sent,
/// so the bar made after it was said knows.
static BAR_SHOWN: AtomicBool = AtomicBool::new(true);

/// The dock's settings, kept the same way.
static PENDING_DOCK: std::sync::Mutex<Option<DockSettings>> = std::sync::Mutex::new(None);

/// How the bar writes the time and the date, read by its clock as it ticks.
static BAR_SETTINGS: std::sync::Mutex<Option<BarSettings>> = std::sync::Mutex::new(None);

/// Whether the shell's theme is light, for every window made after it was
/// said as well as those already up.
static LIGHT: AtomicBool = AtomicBool::new(false);

/// How the bar writes the time and the date.
#[derive(Debug, Clone, PartialEq)]
pub struct BarSettings {
    /// `HH:mm`, `HH:mm:ss` or `hh:mm AP`, as `time.format` has it.
    pub time_format: String,
    /// Whether the shell is in Japanese: the date reads `10月19日(月)` rather
    /// than `Mon 19 Oct`.
    pub japanese: bool,
}

thread_local! {
    /// The bar, on its own thread, for what [`update`] sends there.
    static BAR: RefCell<Option<slint::Weak<Bar>>> = const { RefCell::new(None) };
    /// The tray icons as the shell sent them, which a click hands back.
    static TRAY: RefCell<Vec<Value>> = const { RefCell::new(Vec::new()) };
    /// The readout, made the first time it is shown, and what hides it.
    static OSD: RefCell<Option<Osd>> = const { RefCell::new(None) };
    static OSD_TIMER: slint::Timer = slint::Timer::default();
}

/// Something pressed on a native surface, for the shell to act on.
#[derive(Debug)]
pub enum Action {
    /// `sidebarLeft`, `sidebarRight`, `wallpaperSelector`, `settings` or
    /// `randomWallpaper`.
    Open(String),
    /// `playPause` or `next`.
    Media(String),
    /// A tray icon, as the shell sent it, and whether it was the right button.
    Tray(Value, bool),
    /// A notification closed or swiped away.
    Dismiss(u32),
    /// A dock icon clicked: the application, as the shell sent it.
    DockActivate(Value),
    /// A dock icon to pin, by its executable, or to unpin.
    DockPin(String, bool),
    /// A pinned icon dragged onto another's place, both by executable.
    DockMove(String, String),
}

/// The bar's place on the screen.
pub type Place = Box<dyn FnOnce(isize, i32) -> Option<(i32, i32, i32, i32)> + Send>;

/// What the bar needs from the shell.
pub struct BarHooks {
    /// Called once the window exists, with its handle and the height it wants
    /// in physical pixels; reserves the edge and answers the rectangle the bar
    /// is to occupy.
    pub place: Place,
}

/// The last of each reading the shell has sent.
pub type Latest = Box<dyn FnOnce() -> Vec<(String, Value)> + Send>;

/// Starts the native surfaces' thread, with the bar when there is one.
/// `latest` is called once the surfaces can take readings, for the last of each
/// the shell has sent: anything sent before then is not seen. `act` is called
/// with whatever is pressed on any of them.
pub fn spawn(bar: Option<BarHooks>, latest: Latest, act: Box<dyn Fn(Action) + Send>) {
    RUNNING.store(true, Ordering::Relaxed);
    let started = std::thread::Builder::new()
        .name("bw-native".to_owned())
        .spawn(move || {
            if let Err(error) = run(bar, latest, act) {
                tracing::error!(%error, "the native surfaces stopped");
            }
        });
    if let Err(error) = started {
        tracing::error!(%error, "could not start the native surfaces");
    }
}

/// Hands the bar a reading — the same event name and payload the webview
/// surfaces are sent. Any thread; a no-op when nothing native is running.
pub fn update(event: &str, value: &Value) {
    if !RUNNING.load(Ordering::Relaxed) {
        return;
    }
    let event = event.to_owned();
    let value = value.clone();
    let _ = slint::invoke_from_event_loop(move || {
        BAR.with(|bar| {
            if let Some(bar) = bar.borrow().as_ref().and_then(|bar| bar.upgrade()) {
                apply(&bar, &event, &value);
            }
        });
        dock::apply(&event, &value);
    });
}

/// Shows or hides the bar. Any thread.
pub fn bar_shown(shown: bool) {
    BAR_SHOWN.store(shown, Ordering::Relaxed);
    if RUNNING.load(Ordering::Relaxed) {
        let _ = slint::invoke_from_event_loop(move || {
            BAR.with(|bar| {
                if let Some(bar) = bar.borrow().as_ref().and_then(|bar| bar.upgrade()) {
                    let _ = if shown { bar.show() } else { bar.hide() };
                }
            });
        });
    }
}

/// How the bar writes the time and the date. Any thread; the clock picks it
/// up on its next tick.
pub fn bar(settings: BarSettings) {
    if let Ok(mut kept) = BAR_SETTINGS.lock() {
        *kept = Some(settings);
    }
}

/// Whether the shell's theme is light. Any thread.
pub fn light(light: bool) {
    LIGHT.store(light, Ordering::Relaxed);
    if RUNNING.load(Ordering::Relaxed) {
        let _ = slint::invoke_from_event_loop(move || {
            BAR.with(|bar| {
                if let Some(bar) = bar.borrow().as_ref().and_then(|bar| bar.upgrade()) {
                    bar.global::<Palette>().set_light(light);
                }
            });
            OSD.with(|osd| {
                if let Some(osd) = osd.borrow().as_ref() {
                    osd.global::<Palette>().set_light(light);
                }
            });
            toasts::repaint(light);
            dock::repaint(light);
        });
    }
}

fn is_light() -> bool {
    LIGHT.load(Ordering::Relaxed)
}

/// The dock's settings, as they are now: made, changed, or put away. Any
/// thread.
pub fn dock(settings: DockSettings) {
    if let Ok(mut pending) = PENDING_DOCK.lock() {
        *pending = Some(settings);
    }
    if RUNNING.load(Ordering::Relaxed) {
        let _ = slint::invoke_from_event_loop(configure_pending_dock);
    }
}

fn configure_pending_dock() {
    let pending = PENDING_DOCK
        .lock()
        .ok()
        .and_then(|mut pending| pending.take());
    if let Some(settings) = pending {
        dock::configure(settings);
    }
}

/// Shows the volume or brightness readout for `timeout`, or for longer if
/// another change comes first. `kind` is `volume` or `brightness`; `value` is
/// a percentage. Any thread.
pub fn osd(kind: &str, value: f32, muted: bool, timeout: Duration, bottom: bool) {
    if !RUNNING.load(Ordering::Relaxed) {
        return;
    }
    let volume = kind == "volume";
    let _ = slint::invoke_from_event_loop(move || {
        if let Err(error) = show_osd(volume, value, muted, timeout, bottom) {
            tracing::warn!(%error, "could not show the readout");
        }
    });
}

/// Shows the notification banners for this list — the same payload the
/// webview surfaces are sent — with these settings. Any thread.
pub fn toasts(list: Value, settings: ToastSettings) {
    if let Ok(mut pending) = PENDING_TOASTS.lock() {
        *pending = Some((list, settings));
    }
    if RUNNING.load(Ordering::Relaxed) {
        let _ = slint::invoke_from_event_loop(show_pending_toasts);
    }
}

fn show_pending_toasts() {
    let pending = PENDING_TOASTS
        .lock()
        .ok()
        .and_then(|mut pending| pending.take());
    if let Some((list, settings)) = pending {
        toasts::show(&list, settings);
    }
}

fn run(
    hooks: Option<BarHooks>,
    latest: Latest,
    act: Box<dyn Fn(Action) + Send>,
) -> Result<(), slint::PlatformError> {
    // Drawn on the CPU: OpenGL took five seconds to come up on a slow machine,
    // and surfaces this size are cheap to draw either way. Never activated and
    // never on the taskbar, from birth.
    let selector = slint::BackendSelector::new()
        .backend_name("winit".into())
        .renderer_name("software".into());
    #[cfg(windows)]
    let selector = selector.with_winit_window_attributes_hook(|attributes| {
        use slint::winit_030::winit::platform::windows::WindowAttributesExtWindows;
        attributes.with_active(false).with_skip_taskbar(true)
    });
    selector.select()?;

    let act: Rc<dyn Fn(Action)> = Rc::from(act as Box<dyn Fn(Action)>);
    toasts::set_act(act.clone());
    dock::set_act(act.clone());
    show_pending_toasts();

    let latest = latest();
    for (event, value) in &latest {
        dock::apply(event, value);
    }
    configure_pending_dock();

    // Kept alive for as long as the loop runs.
    let _bar = match hooks {
        Some(hooks) => Some(start_bar(hooks, &latest, act)?),
        None => None,
    };
    // Hiding the readout must not end the loop, as the last window closing
    // would.
    slint::run_event_loop_until_quit()
}

fn start_bar(
    hooks: BarHooks,
    latest: &[(String, Value)],
    act: Rc<dyn Fn(Action)>,
) -> Result<(Bar, slint::Timer), slint::PlatformError> {
    let started = Instant::now();
    let bar = Bar::new()?;
    bar.global::<Palette>().set_light(is_light());
    set_icons(&bar);

    let (width, height) = screen_strip();
    bar.window()
        .set_position(slint::PhysicalPosition::new(0, 0));
    bar.window()
        .set_size(slint::PhysicalSize::new(width, height as u32));

    let acting = act.clone();
    bar.on_clicked(move |what| {
        let what = what.to_string();
        acting(match what.as_str() {
            "playPause" | "next" => Action::Media(what),
            _ => Action::Open(what),
        });
    });
    bar.on_tray_clicked(move |index, secondary| {
        let icon = TRAY.with(|tray| tray.borrow().get(index as usize).cloned());
        if let Some(icon) = icon {
            act(Action::Tray(icon, secondary));
        }
    });

    BAR.with(|slot| *slot.borrow_mut() = Some(bar.as_weak()));
    for (event, value) in latest {
        apply(&bar, event, value);
    }

    let tick = slint::Timer::default();
    let weak = bar.as_weak();
    let mut place = Some(hooks.place);
    tick.start(
        slint::TimerMode::Repeated,
        Duration::from_millis(250),
        move || {
            let Some(bar) = weak.upgrade() else { return };
            // The window exists only once the event loop runs.
            if let Some(hwnd) = place.as_ref().and_then(|_| window_of(bar.window())) {
                let place = place.take().expect("checked above");
                settle(hwnd, place(hwnd, height));
                tracing::info!(
                    ms = started.elapsed().as_millis() as u64,
                    "the native bar is on screen"
                );
            }
            show_clock(&bar);
        },
    );
    show_clock(&bar);

    if BAR_SHOWN.load(Ordering::Relaxed) {
        bar.show()?;
    }
    Ok((bar, tick))
}

fn show_osd(
    volume: bool,
    value: f32,
    muted: bool,
    timeout: Duration,
    bottom: bool,
) -> Result<(), slint::PlatformError> {
    let first = OSD.with(|slot| slot.borrow().is_none());
    if first {
        let made = Osd::new()?;
        made.global::<Palette>().set_light(is_light());
        OSD.with(|slot| *slot.borrow_mut() = Some(made));
    }
    OSD.with(|slot| -> Result<(), slint::PlatformError> {
        let slot = slot.borrow();
        let Some(osd) = slot.as_ref() else {
            return Ok(());
        };
        let percent = value.clamp(0.0, 100.0);
        let name = match (volume, muted || percent <= 0.0, percent) {
            (true, true, _) => "volume_off",
            (true, false, p) if p < 50.0 => "volume_down",
            (true, false, _) => "volume_up",
            (false, _, p) if p < 40.0 => "brightness_low",
            (false, _, _) => "brightness_high",
        };
        osd.set_glyph(icon(name, 20.0, false));
        osd.set_level(percent / 100.0);
        osd.set_value(format!("{}", percent.round()).into());
        osd.set_muted(volume && muted);

        // Centred, under the bar, or above the bottom edge.
        let scale = osd.window().scale_factor();
        let (screen_width, screen_height) = screen_size();
        let (width, height) = ((320.0 * scale) as i32, (46.0 * scale) as i32);
        let margin = (8.0 * scale) as i32;
        let y = if bottom {
            screen_height - height - margin
        } else {
            (HEIGHT * scale) as i32 + margin
        };
        osd.window()
            .set_position(slint::PhysicalPosition::new((screen_width - width) / 2, y));
        osd.show()?;
        if first {
            // Once the window exists, which is after the loop has had a turn.
            let weak = osd.as_weak();
            slint::Timer::single_shot(Duration::ZERO, move || {
                if let Some(hwnd) = weak.upgrade().and_then(|osd| window_of(osd.window())) {
                    settle(hwnd, None);
                    round_corners(hwnd);
                }
            });
        }
        Ok(())
    })?;

    OSD_TIMER.with(|timer| {
        timer.start(slint::TimerMode::SingleShot, timeout, || {
            OSD.with(|slot| {
                if let Some(osd) = slot.borrow().as_ref() {
                    let _ = osd.hide();
                }
            });
        });
    });
    Ok(())
}

/// An icon from the shared set, drawn as the webview's `Symbol` draws it: a
/// hairline near 1.5 px at `size`, thicker when `filled`.
fn icon(name: &str, size: f32, filled: bool) -> slint::Image {
    let Some(body) = icon_body(name) else {
        return slint::Image::default();
    };
    let stroke = 1.5 * 24.0 / size * if filled { 1.35 } else { 1.0 };
    let svg = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="#fff" stroke-width="{stroke}" stroke-linecap="round" stroke-linejoin="round">{body}</svg>"##
    );
    slint::Image::load_from_svg_data(svg.as_bytes()).unwrap_or_default()
}

fn set_icons(bar: &Bar) {
    let icons = bar.global::<Icons>();
    icons.set_left_panel(icon("left_panel_open", 16.0, false));
    icons.set_right_panel(icon("right_panel_open", 16.0, false));
    icons.set_desktop(icon("desktop_windows", 16.0, false));
    icons.set_wallpaper(icon("wallpaper", 16.0, false));
    icons.set_speed(icon("speed", 16.0, false));
    icons.set_memory(icon("memory", 16.0, false));
    icons.set_storage(icon("storage", 16.0, false));
    icons.set_down(icon("arrow_downward", 14.0, false));
    icons.set_up(icon("arrow_upward", 14.0, false));
    icons.set_battery(icon("battery_full", 16.0, false));
    icons.set_battery_charging(icon("battery_charging_full", 16.0, false));
    icons.set_photos(icon("photo_library", 16.0, false));
    icons.set_shuffle(icon("shuffle", 16.0, false));
    icons.set_settings(icon("settings", 16.0, false));
    icons.set_play(icon("play_arrow", 16.0, false));
    icons.set_pause(icon("pause", 16.0, false));
    icons.set_next(icon("skip_next", 16.0, false));
}

fn show_clock(bar: &Bar) {
    let settings = BAR_SETTINGS
        .lock()
        .ok()
        .and_then(|kept| kept.clone())
        .unwrap_or(BarSettings {
            time_format: "HH:mm".to_owned(),
            japanese: true,
        });
    let (time, date) = clock(&chrono::Local::now(), &settings);
    if bar.get_time() != time.as_str() {
        bar.set_time(time.into());
    }
    if bar.get_date() != date.as_str() {
        bar.set_date(date.into());
    }
    let desktop = if settings.japanese {
        "デスクトップ"
    } else {
        "Desktop"
    };
    if bar.get_desktop_label() != desktop {
        bar.set_desktop_label(desktop.into());
    }
}

/// The time and the date as the webview bar wrote them: the three formats the
/// settings offer, anything else read as `HH:mm`, and the date as a Mac's menu
/// bar has it in each language.
fn clock(now: &(impl Datelike + Timelike), settings: &BarSettings) -> (String, String) {
    let (hour, minute) = (now.hour(), now.minute());
    let time = match settings.time_format.as_str() {
        "HH:mm:ss" => format!("{hour:02}:{minute:02}:{:02}", now.second()),
        "hh:mm AP" => {
            let (pm, twelve) = now.hour12();
            let twelve = format!("{twelve:02}:{minute:02}");
            match (settings.japanese, pm) {
                (true, true) => format!("午後{twelve}"),
                (true, false) => format!("午前{twelve}"),
                (false, true) => format!("{twelve} PM"),
                (false, false) => format!("{twelve} AM"),
            }
        }
        _ => format!("{hour:02}:{minute:02}"),
    };
    let weekday = now.weekday().num_days_from_monday() as usize;
    let date = if settings.japanese {
        const DAYS: [&str; 7] = ["月", "火", "水", "木", "金", "土", "日"];
        format!("{}月{}日({})", now.month(), now.day(), DAYS[weekday])
    } else {
        const DAYS: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
        const MONTHS: [&str; 12] = [
            "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
        ];
        format!(
            "{} {} {}",
            DAYS[weekday],
            now.day(),
            MONTHS[now.month0() as usize]
        )
    };
    (time, date)
}

/// Shows one reading. The names are the shell's `event` constants.
fn apply(bar: &Bar, event: &str, value: &Value) {
    match event {
        "bw://resources" => {
            #[derive(Deserialize)]
            struct Resources {
                cpu: f32,
                memory: f32,
                disk: f32,
            }
            if let Ok(r) = Resources::deserialize(value) {
                bar.set_cpu(format!("{:.0}%", r.cpu).into());
                bar.set_memory(format!("{:.0}%", r.memory).into());
                bar.set_disk(format!("{:.0}%", r.disk).into());
            }
        }
        "bw://network" => {
            #[derive(Deserialize)]
            struct Network {
                down: f64,
                up: f64,
            }
            if let Ok(n) = Network::deserialize(value) {
                bar.set_down(rate(n.down).into());
                bar.set_up(rate(n.up).into());
            }
        }
        "bw://battery" => {
            #[derive(Deserialize)]
            struct Battery {
                present: bool,
                percent: f32,
                charging: bool,
            }
            if let Ok(b) = Battery::deserialize(value) {
                let shown = if b.present {
                    format!("{:.0}%", b.percent)
                } else {
                    String::new()
                };
                bar.set_battery(shown.into());
                bar.set_charging(b.charging);
            }
        }
        "bw://weather" => {
            #[derive(Deserialize)]
            struct Weather {
                temperature: f64,
                icon: String,
            }
            if let Ok(w) = Weather::deserialize(value) {
                bar.set_weather_icon(icon(&w.icon, 17.0, true));
                bar.set_weather(format!("{}°", w.temperature.round()).into());
            }
        }
        "bw://media" => {
            #[derive(Deserialize)]
            struct Media {
                title: String,
                playing: bool,
            }
            if let Ok(m) = Media::deserialize(value) {
                bar.set_media_title(m.title.into());
                bar.set_media_playing(m.playing);
            }
        }
        "bw://active-window" => {
            #[derive(Deserialize)]
            struct Active {
                title: String,
            }
            if let Ok(a) = Active::deserialize(value) {
                bar.set_active_title(a.title.trim().into());
            }
        }
        "bw://tray" => {
            let shown: Vec<Value> = value
                .as_array()
                .into_iter()
                .flatten()
                .filter(|icon| !icon["hidden"].as_bool().unwrap_or(false))
                .cloned()
                .collect();
            let items: Vec<TrayItem> = shown
                .iter()
                .map(|icon| TrayItem {
                    icon: icon["icon"]
                        .as_str()
                        .filter(|path| !path.is_empty())
                        .and_then(|path| slint::Image::load_from_path(path.as_ref()).ok())
                        .unwrap_or_default(),
                })
                .collect();
            TRAY.with(|tray| *tray.borrow_mut() = shown);
            bar.set_tray(ModelRc::new(VecModel::from(items)));
        }
        "bw://workspaces" => {
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Workspace {
                display_name: String,
                focused: bool,
                populated: bool,
            }
            #[derive(Deserialize)]
            struct Workspaces {
                source: Option<String>,
                workspaces: Vec<Workspace>,
            }
            // Without GlazeWM or komorebi there is nothing to show, rather
            // than a fake single workspace.
            let items: Vec<WorkspaceItem> = Workspaces::deserialize(value)
                .ok()
                .filter(|w| w.source.is_some())
                .map(|w| w.workspaces)
                .unwrap_or_default()
                .into_iter()
                .map(|w| WorkspaceItem {
                    name: w.display_name.into(),
                    focused: w.focused,
                    populated: w.populated,
                })
                .collect();
            if bar.get_workspaces().row_count() != 0 || !items.is_empty() {
                bar.set_workspaces(ModelRc::new(VecModel::from(items)));
            }
        }
        _ => {}
    }
}

/// Bytes a second, as the webview bar's `formatRate` writes them: "512B",
/// "5.5K", "18K", "1.3M".
fn rate(bytes: f64) -> String {
    const UNITS: [&str; 4] = ["B", "K", "M", "G"];
    let mut value = bytes.max(0.0);
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if value < 10.0 && unit != 0 {
        format!("{value:.1}{}", UNITS[unit])
    } else {
        format!("{}{}", value.round(), UNITS[unit])
    }
}

/// The primary screen, in physical pixels.
fn screen_size() -> (i32, i32) {
    #[cfg(windows)]
    unsafe {
        use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};
        (GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN))
    }
    #[cfg(not(windows))]
    (1920, 1080)
}

/// Has Windows round a window's corners and give it its shadow, as it does a
/// menu's: the readout is a block, and drawing either with a transparent
/// window would need a renderer that can.
fn round_corners(hwnd: isize) {
    #[cfg(windows)]
    unsafe {
        use windows::Win32::Foundation::HWND;
        use windows::Win32::Graphics::Dwm::{
            DwmSetWindowAttribute, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND,
        };
        let preference = DWMWCP_ROUND;
        let _ = DwmSetWindowAttribute(
            HWND(hwnd as _),
            DWMWA_WINDOW_CORNER_PREFERENCE,
            std::ptr::addr_of!(preference).cast(),
            std::mem::size_of_val(&preference) as u32,
        );
    }
    #[cfg(not(windows))]
    let _ = hwnd;
}

/// The top strip of the primary screen, in physical pixels.
fn screen_strip() -> (u32, i32) {
    #[cfg(windows)]
    unsafe {
        use windows::Win32::UI::HiDpi::GetDpiForSystem;
        use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN};
        let scale = GetDpiForSystem() as f32 / 96.0;
        (
            GetSystemMetrics(SM_CXSCREEN) as u32,
            (HEIGHT * scale).round() as i32,
        )
    }
    #[cfg(not(windows))]
    (1920, HEIGHT as i32)
}

fn window_of(window: &slint::Window) -> Option<isize> {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    match window.window_handle().window_handle().ok()?.as_raw() {
        RawWindowHandle::Win32(handle) => Some(handle.hwnd.get()),
        _ => None,
    }
}

/// Keeps the bar off the taskbar and out of the foreground, and moves it to
/// the rectangle it was granted.
fn settle(hwnd: isize, granted: Option<(i32, i32, i32, i32)>) {
    #[cfg(windows)]
    unsafe {
        use windows::Win32::Foundation::HWND;
        use windows::Win32::UI::WindowsAndMessaging::{
            GetWindowLongPtrW, SetWindowLongPtrW, SetWindowPos, GWL_EXSTYLE, HWND_TOPMOST,
            SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
        };

        let hwnd = HWND(hwnd as _);
        let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        SetWindowLongPtrW(
            hwnd,
            GWL_EXSTYLE,
            style | (WS_EX_TOOLWINDOW.0 | WS_EX_NOACTIVATE.0) as isize,
        );
        let _ = match granted {
            Some((x, y, width, height)) => {
                SetWindowPos(hwnd, HWND_TOPMOST, x, y, width, height, SWP_NOACTIVATE)
            }
            None => SetWindowPos(
                hwnd,
                HWND_TOPMOST,
                0,
                0,
                0,
                0,
                SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOSIZE,
            ),
        };
    }
    #[cfg(not(windows))]
    let _ = (hwnd, granted);
}

#[cfg(test)]
mod tests {
    #[test]
    fn rates_read_as_the_webview_bar_writes_them() {
        assert_eq!(super::rate(512.0), "512B");
        assert_eq!(super::rate(5.5 * 1024.0), "5.5K");
        assert_eq!(super::rate(18.0 * 1024.0), "18K");
        assert_eq!(super::rate(1.3 * 1024.0 * 1024.0), "1.3M");
    }

    #[test]
    fn the_clock_reads_as_the_webview_bar_wrote_it() {
        use super::{clock, BarSettings};
        let at = chrono::NaiveDate::from_ymd_opt(2026, 10, 19)
            .unwrap()
            .and_hms_opt(14, 5, 9)
            .unwrap();
        let read = |format: &str, japanese| {
            clock(
                &at,
                &BarSettings {
                    time_format: format.to_owned(),
                    japanese,
                },
            )
        };
        assert_eq!(read("HH:mm", true), ("14:05".into(), "10月19日(月)".into()));
        assert_eq!(
            read("HH:mm:ss", false),
            ("14:05:09".into(), "Mon 19 Oct".into())
        );
        assert_eq!(read("hh:mm AP", false).0, "02:05 PM");
        assert_eq!(read("hh:mm AP", true).0, "午後02:05");
        assert_eq!(read("something else", false).0, "14:05");
    }

    #[test]
    fn the_shared_icon_set_is_read() {
        assert!(super::icon_body("speed").is_some());
        assert!(super::icon_body("weather_snowy").is_some());
        assert!(super::icon_body("not an icon").is_none());
    }
}
