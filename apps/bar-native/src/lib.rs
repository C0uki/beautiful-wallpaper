//! The bar, drawn natively with Slint instead of in a webview.
//!
//! Without Chromium behind it the bar is on screen in about a second and costs
//! about 10 MB, where the webview bar needs WebView2's browser and GPU
//! processes — some 300 MB on their own — before it can draw at all.
//!
//! It runs on a thread of its own with its own event loop, beside Tauri's: on
//! Windows Slint's winit backend allows that, and its windows never touch the
//! shell's. It reads nothing itself but the clock. The shell hands it the
//! readings it sends the webview surfaces, through [`update`], and hears what
//! was pressed through the callback given to [`spawn`].

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use chrono::{Datelike, Timelike};
use serde::Deserialize;
use serde_json::Value;
use slint::{ComponentHandle, Model, ModelRc, VecModel};

slint::include_modules!();
include!(concat!(env!("OUT_DIR"), "/icons.rs"));

/// The webview bar's height at 100%.
const HEIGHT: f32 = 30.0;

/// Whether the bar was started, so [`update`] costs nothing when it was not.
static RUNNING: AtomicBool = AtomicBool::new(false);

thread_local! {
    /// The bar, on its own thread, for what [`update`] sends there.
    static BAR: RefCell<Option<slint::Weak<Bar>>> = const { RefCell::new(None) };
    /// The tray icons as the shell sent them, which a click hands back.
    static TRAY: RefCell<Vec<Value>> = const { RefCell::new(Vec::new()) };
}

/// Something pressed on the bar, for the shell to act on.
#[derive(Debug)]
pub enum Action {
    /// `sidebarLeft`, `sidebarRight`, `wallpaperSelector`, `settings` or
    /// `randomWallpaper`.
    Open(String),
    /// `playPause` or `next`.
    Media(String),
    /// A tray icon, as the shell sent it, and whether it was the right button.
    Tray(Value, bool),
}

/// Starts the bar on a thread of its own.
///
/// `place` is called on that thread once the window exists, with its handle and
/// the height it wants in physical pixels; it reserves the edge and answers the
/// rectangle the bar is to occupy. `latest` is called once the bar can take
/// readings, for the last of each the shell has sent: anything sent before then
/// is not seen. `act` is called with what was pressed.
pub fn spawn(
    place: impl FnOnce(isize, i32) -> Option<(i32, i32, i32, i32)> + Send + 'static,
    latest: impl FnOnce() -> Vec<(String, Value)> + Send + 'static,
    act: impl Fn(Action) + Send + 'static,
) {
    RUNNING.store(true, Ordering::Relaxed);
    let started = std::thread::Builder::new()
        .name("bw-bar".to_owned())
        .spawn(move || {
            if let Err(error) = run(place, latest, act) {
                tracing::error!(%error, "the native bar stopped");
            }
        });
    if let Err(error) = started {
        tracing::error!(%error, "could not start the native bar");
    }
}

/// Hands the bar a reading — the same event name and payload the webview
/// surfaces are sent. Any thread; a no-op when the bar is not running.
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
    });
}

fn run(
    place: impl FnOnce(isize, i32) -> Option<(i32, i32, i32, i32)> + 'static,
    latest: impl FnOnce() -> Vec<(String, Value)>,
    act: impl Fn(Action) + 'static,
) -> Result<(), slint::PlatformError> {
    let started = Instant::now();
    // Drawn on the CPU: OpenGL took five seconds to come up on a slow machine,
    // and a strip this size is cheap to draw either way.
    slint::BackendSelector::new()
        .backend_name("winit".into())
        .renderer_name("software".into())
        .select()?;
    let bar = Bar::new()?;
    set_icons(&bar);

    let (width, height) = screen_strip();
    bar.window()
        .set_position(slint::PhysicalPosition::new(0, 0));
    bar.window()
        .set_size(slint::PhysicalSize::new(width, height as u32));

    let act = Rc::new(act);
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
    for (event, value) in latest() {
        apply(&bar, &event, &value);
    }

    let tick = slint::Timer::default();
    let weak = bar.as_weak();
    let mut place = Some(place);
    tick.start(
        slint::TimerMode::Repeated,
        Duration::from_millis(250),
        move || {
            let Some(bar) = weak.upgrade() else { return };
            // The window exists only once the event loop runs.
            if let Some(hwnd) = place.as_ref().and_then(|_| window_of(&bar)) {
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

    bar.show()?;
    slint::run_event_loop()
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
    let now = chrono::Local::now();
    let time = format!("{:02}:{:02}:{:02}", now.hour(), now.minute(), now.second());
    if bar.get_time() != time {
        bar.set_time(time.into());
        // ponytail: Japanese only; the locale's own format once the bar
        // follows the shell's language.
        const DAYS: [&str; 7] = ["月", "火", "水", "木", "金", "土", "日"];
        let day = DAYS[now.weekday().num_days_from_monday() as usize];
        bar.set_date(format!("{}月{}日({day})", now.month(), now.day()).into());
    }
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

fn window_of(bar: &Bar) -> Option<isize> {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    match bar.window().window_handle().window_handle().ok()?.as_raw() {
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
    fn the_shared_icon_set_is_read() {
        assert!(super::icon_body("speed").is_some());
        assert!(super::icon_body("weather_snowy").is_some());
        assert!(super::icon_body("not an icon").is_none());
    }
}
