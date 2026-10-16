//! The bar, drawn natively with Slint instead of in a webview.
//!
//! A prototype, to measure what the bar costs without Chromium behind it — its
//! memory, how soon it is on screen, and whether it can look the same. It runs
//! on its own, beside or instead of the shell, and reads what it shows itself.
//! `taskkill /im bw-bar.exe` closes it and gives its edge back.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::time::{Duration, Instant};

use chrono::{Datelike, Timelike};
use sysinfo::{Disks, Networks, System};

slint::include_modules!();

/// The webview bar's height at 100%.
const HEIGHT: f32 = 30.0;

fn main() -> Result<(), slint::PlatformError> {
    let started = Instant::now();
    let bar = Bar::new()?;

    let (width, height) = screen_strip();
    bar.window()
        .set_position(slint::PhysicalPosition::new(0, 0));
    bar.window()
        .set_size(slint::PhysicalSize::new(width, height));

    bar.on_clicked(|what| {
        let args: &[&str] = match what.as_str() {
            "settings" => &["settings", "open"],
            "wallpaperSelector" => &["wallpaperSelector", "toggle"],
            "randomWallpaper" => &["wallpapers", "random"],
            // ponytail: the sidebars have no command line way in yet.
            _ => return,
        };
        if let Some(bw) = std::env::var_os("LOCALAPPDATA").map(|dir| {
            std::path::Path::new(&dir)
                .join("beautiful-wallpaper")
                .join("bw.exe")
        }) {
            let _ = std::process::Command::new(bw).args(args).spawn();
        }
    });

    let mut readings = Readings::new();
    readings.apply(&bar);

    // ponytail: sampled on the UI thread, a few milliseconds a second; a
    // thread of its own once there is more to read.
    let tick = slint::Timer::default();
    let weak = bar.as_weak();
    let mut placed = false;
    tick.start(
        slint::TimerMode::Repeated,
        Duration::from_millis(100),
        move || {
            let Some(bar) = weak.upgrade() else { return };
            if !placed {
                // The window exists only once the event loop runs.
                placed = reserve_edge(width, height);
                if placed {
                    eprintln!(
                        "bw-bar: on screen {} ms after start",
                        started.elapsed().as_millis()
                    );
                }
            }
            readings.tick(&bar);
        },
    );

    bar.show()?;
    slint::run_event_loop()?;
    release_edge();
    Ok(())
}

/// The top strip of the primary screen, in physical pixels.
fn screen_strip() -> (u32, u32) {
    #[cfg(windows)]
    unsafe {
        use windows::Win32::UI::HiDpi::GetDpiForSystem;
        use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN};
        let scale = GetDpiForSystem() as f32 / 96.0;
        (
            GetSystemMetrics(SM_CXSCREEN) as u32,
            (HEIGHT * scale).round() as u32,
        )
    }
    #[cfg(not(windows))]
    (1920, HEIGHT as u32)
}

#[cfg(windows)]
fn our_window() -> Option<windows::Win32::Foundation::HWND> {
    use windows::core::w;
    use windows::Win32::UI::WindowsAndMessaging::FindWindowW;
    unsafe { FindWindowW(None, w!("bw-bar-native")).ok() }
}

/// Keeps the bar off the taskbar and out of the foreground, and has Windows
/// keep maximised windows clear of it. False until the window exists.
fn reserve_edge(width: u32, height: u32) -> bool {
    #[cfg(windows)]
    unsafe {
        use windows::Win32::Foundation::RECT;
        use windows::Win32::UI::Shell::{
            SHAppBarMessage, ABE_TOP, ABM_NEW, ABM_QUERYPOS, ABM_SETPOS, APPBARDATA,
        };
        use windows::Win32::UI::WindowsAndMessaging::{
            GetWindowLongPtrW, SetWindowLongPtrW, SetWindowPos, GWL_EXSTYLE, HWND_TOPMOST,
            SWP_NOACTIVATE, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
        };

        let Some(hwnd) = our_window() else {
            return false;
        };
        let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        SetWindowLongPtrW(
            hwnd,
            GWL_EXSTYLE,
            style | (WS_EX_TOOLWINDOW.0 | WS_EX_NOACTIVATE.0) as isize,
        );

        let mut data = APPBARDATA {
            cbSize: std::mem::size_of::<APPBARDATA>() as u32,
            hWnd: hwnd,
            uEdge: ABE_TOP,
            rc: RECT {
                left: 0,
                top: 0,
                right: width as i32,
                bottom: height as i32,
            },
            ..Default::default()
        };
        SHAppBarMessage(ABM_NEW, &mut data);
        SHAppBarMessage(ABM_QUERYPOS, &mut data);
        data.rc.bottom = data.rc.top + height as i32;
        SHAppBarMessage(ABM_SETPOS, &mut data);
        let rc = data.rc;
        let _ = SetWindowPos(
            hwnd,
            HWND_TOPMOST,
            rc.left,
            rc.top,
            rc.right - rc.left,
            rc.bottom - rc.top,
            SWP_NOACTIVATE,
        );
    }
    #[cfg(not(windows))]
    let _ = (width, height);
    true
}

fn release_edge() {
    #[cfg(windows)]
    unsafe {
        use windows::Win32::UI::Shell::{SHAppBarMessage, ABM_REMOVE, APPBARDATA};
        if let Some(hwnd) = our_window() {
            let mut data = APPBARDATA {
                cbSize: std::mem::size_of::<APPBARDATA>() as u32,
                hWnd: hwnd,
                ..Default::default()
            };
            SHAppBarMessage(ABM_REMOVE, &mut data);
        }
    }
}

/// What the bar shows, read from the system.
struct Readings {
    system: System,
    networks: Networks,
    disks: Disks,
    last_sample: Instant,
    last_title: String,
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

        let title = active_title();
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

fn active_title() -> String {
    #[cfg(windows)]
    unsafe {
        use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowTextW};
        let mut buffer = [0u16; 256];
        let length = GetWindowTextW(GetForegroundWindow(), &mut buffer);
        String::from_utf16_lossy(&buffer[..length.max(0) as usize])
    }
    #[cfg(not(windows))]
    String::new()
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
