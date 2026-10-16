//! The bar, drawn natively with Slint instead of in a webview.
//!
//! Without Chromium behind it the bar is on screen in about a second and costs
//! about 10 MB, where the webview bar needs WebView2's browser and GPU
//! processes — some 300 MB on their own — before it can draw at all.
//!
//! It runs on a thread of its own with its own event loop, beside Tauri's: on
//! Windows Slint's winit backend allows that, and its windows never touch the
//! shell's. The shell is reached only through the two callbacks given to
//! [`spawn`].

use std::time::{Duration, Instant};

use chrono::{Datelike, Timelike};
use sysinfo::{Disks, Networks, System};

slint::include_modules!();

/// The webview bar's height at 100%.
const HEIGHT: f32 = 30.0;

/// Starts the bar on a thread of its own.
///
/// `place` is called on that thread once the window exists, with its handle and
/// the height it wants in physical pixels; it reserves the edge and answers the
/// rectangle the bar is to occupy. `clicked` is called with what was pressed:
/// `sidebarLeft`, `sidebarRight`, `wallpaperSelector`, `randomWallpaper` or
/// `settings`.
pub fn spawn(
    place: impl FnOnce(isize, i32) -> Option<(i32, i32, i32, i32)> + Send + 'static,
    clicked: impl Fn(&str) + Send + 'static,
) {
    let started = std::thread::Builder::new()
        .name("bw-bar".to_owned())
        .spawn(move || {
            if let Err(error) = run(place, clicked) {
                tracing::error!(%error, "the native bar stopped");
            }
        });
    if let Err(error) = started {
        tracing::error!(%error, "could not start the native bar");
    }
}

fn run(
    place: impl FnOnce(isize, i32) -> Option<(i32, i32, i32, i32)> + 'static,
    clicked: impl Fn(&str) + 'static,
) -> Result<(), slint::PlatformError> {
    let started = Instant::now();
    // Drawn on the CPU: OpenGL took five seconds to come up on a slow machine,
    // and a strip this size is cheap to draw either way.
    slint::BackendSelector::new()
        .backend_name("winit".into())
        .renderer_name("software".into())
        .select()?;
    let bar = Bar::new()?;

    let (width, height) = screen_strip();
    bar.window()
        .set_position(slint::PhysicalPosition::new(0, 0));
    bar.window()
        .set_size(slint::PhysicalSize::new(width, height as u32));
    bar.on_clicked(move |what| clicked(&what));

    let mut readings = Readings::new();
    readings.apply(&bar);

    // ponytail: sampled on the bar's thread, a few milliseconds a second; the
    // shell's own readings once the rest of the widgets come over.
    let tick = slint::Timer::default();
    let weak = bar.as_weak();
    let mut place = Some(place);
    tick.start(
        slint::TimerMode::Repeated,
        Duration::from_millis(100),
        move || {
            let Some(bar) = weak.upgrade() else { return };
            // The window exists only once the event loop runs.
            if let Some(hwnd) = place.as_ref().and_then(|_| window_of(&bar)) {
                let place = place.take().expect("checked above");
                settle(hwnd, place(hwnd, height));
                readings.own = hwnd;
                tracing::info!(
                    ms = started.elapsed().as_millis() as u64,
                    "the native bar is on screen"
                );
            }
            readings.tick(&bar);
        },
    );

    bar.show()?;
    slint::run_event_loop()
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

/// What the bar shows, read from the system.
struct Readings {
    system: System,
    networks: Networks,
    disks: Disks,
    last_sample: Instant,
    last_title: String,
    /// The bar's own window, which is not the one the user is working in.
    own: isize,
}

impl Readings {
    fn new() -> Self {
        let mut system = System::new();
        system.refresh_cpu_usage();
        system.refresh_memory();
        Self {
            system,
            networks: Networks::new_with_refreshed_list(),
            disks: Disks::new_with_refreshed_list(),
            last_sample: Instant::now(),
            last_title: String::new(),
            own: 0,
        }
    }

    /// Every tick: the clock and the window title, which change between
    /// samples. The rest once a second, as the webview bar has them.
    fn tick(&mut self, bar: &Bar) {
        let now = chrono::Local::now();
        let time = format!("{:02}:{:02}:{:02}", now.hour(), now.minute(), now.second());
        if bar.get_time() != time {
            bar.set_time(time.into());
            // ponytail: Japanese only; the locale's own format once this is
            // more than a prototype.
            const DAYS: [&str; 7] = ["月", "火", "水", "木", "金", "土", "日"];
            let day = DAYS[now.weekday().num_days_from_monday() as usize];
            bar.set_date(format!("{}月{}日({day})", now.month(), now.day()).into());
        }

        let title = active_title(self.own);
        if title != self.last_title {
            bar.set_active_title(title.as_str().into());
            self.last_title = title;
        }

        if self.last_sample.elapsed() >= Duration::from_secs(1) {
            self.apply(bar);
        }
    }

    fn apply(&mut self, bar: &Bar) {
        let seconds = self.last_sample.elapsed().as_secs_f64().max(0.001);
        self.last_sample = Instant::now();

        self.system.refresh_cpu_usage();
        self.system.refresh_memory();
        bar.set_cpu(format!("{:.0}%", self.system.global_cpu_usage()).into());
        let total = self.system.total_memory().max(1) as f64;
        bar.set_memory(format!("{:.0}%", self.system.used_memory() as f64 / total * 100.0).into());

        self.disks.refresh(true);
        let system_drive = std::env::var("SystemDrive").unwrap_or_else(|_| "C:".into());
        if let Some(disk) = self.disks.iter().find(|disk| {
            disk.mount_point()
                .to_string_lossy()
                .starts_with(&system_drive)
        }) {
            let total = disk.total_space().max(1) as f64;
            let used = total - disk.available_space() as f64;
            bar.set_disk(format!("{:.0}%", used / total * 100.0).into());
        }

        self.networks.refresh(true);
        let (down, up) = self.networks.iter().fold((0, 0), |(down, up), (_, data)| {
            (down + data.received(), up + data.transmitted())
        });
        bar.set_down(rate(down as f64 / seconds).into());
        bar.set_up(rate(up as f64 / seconds).into());

        let (percent, charging) = battery();
        bar.set_battery(
            percent
                .map(|percent| format!("{percent}%"))
                .unwrap_or_default()
                .into(),
        );
        bar.set_charging(charging);
    }
}

/// Bytes a second, as the webview bar writes them: "18K", "1.2M".
fn rate(bytes: f64) -> String {
    if bytes < 1024.0 {
        format!("{bytes:.0}B")
    } else if bytes < 1024.0 * 1024.0 {
        format!("{:.0}K", bytes / 1024.0)
    } else {
        format!("{:.1}M", bytes / 1024.0 / 1024.0)
    }
}

/// The title of the window the user is working in. Empty while that is the
/// bar itself, which only happens before it is told not to take the foreground.
fn active_title(own: isize) -> String {
    #[cfg(windows)]
    unsafe {
        use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowTextW};
        let foreground = GetForegroundWindow();
        if foreground.0 as isize == own {
            return String::new();
        }
        let mut buffer = [0u16; 256];
        let length = GetWindowTextW(foreground, &mut buffer);
        String::from_utf16_lossy(&buffer[..length.max(0) as usize])
    }
    #[cfg(not(windows))]
    {
        let _ = own;
        String::new()
    }
}

/// The charge, when there is a battery, and whether it is on the charger.
fn battery() -> (Option<u8>, bool) {
    #[cfg(windows)]
    unsafe {
        use windows::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
        let mut status = SYSTEM_POWER_STATUS::default();
        if GetSystemPowerStatus(&mut status).is_ok() && status.BatteryLifePercent <= 100 {
            return (Some(status.BatteryLifePercent), status.ACLineStatus == 1);
        }
    }
    (None, false)
}

#[cfg(test)]
mod tests {
    #[test]
    fn rates_read_as_the_webview_bar_writes_them() {
        assert_eq!(super::rate(512.0), "512B");
        assert_eq!(super::rate(18.0 * 1024.0), "18K");
        assert_eq!(super::rate(1.3 * 1024.0 * 1024.0), "1.3M");
    }
}
