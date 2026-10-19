//! The hot corners, as a plain window rather than a webview.
//!
//! A webview only ever caught the pointer here: the strips are invisible, and
//! a page of its own was a renderer process kept alive to notice a mouse. This
//! is the same window — the whole screen, cut down to the strips by its region
//! — with nothing in it, and the pointer handled where Windows reports it.
//!
//! It lives on the main thread, whose loop Tao already pumps, and does what the
//! page did through the same commands.

use std::sync::atomic::{AtomicIsize, Ordering};
use std::sync::OnceLock;

use bw_core::chrome::{Corner, HotCorner};
use parking_lot::Mutex;
use tauri::{AppHandle, Manager};
use windows::core::w;
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, CreateSolidBrush, DeleteObject, EndPaint, FillRect, InvalidateRect, PAINTSTRUCT,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::{TrackMouseEvent, TME_LEAVE, TRACKMOUSEEVENT};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, GetSystemMetrics, LoadCursorW, RegisterClassW,
    SetLayeredWindowAttributes, ShowWindow, IDC_ARROW, LWA_ALPHA, MA_NOACTIVATE, SM_CXSCREEN,
    SM_CYSCREEN, SW_HIDE, SW_SHOWNOACTIVATE, WM_LBUTTONDOWN, WM_MOUSEACTIVATE, WM_MOUSEMOVE,
    WM_MOUSEWHEEL, WM_PAINT, WNDCLASSW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
    WS_EX_TOPMOST, WS_POPUP,
};

use crate::state::AppState;

static APP: OnceLock<AppHandle> = OnceLock::new();
static WINDOW: AtomicIsize = AtomicIsize::new(0);
static CORNERS: Mutex<Vec<HotCorner>> = Mutex::new(Vec::new());
/// The corner the pointer is in, so passing over one fires it once.
static INSIDE: Mutex<Option<Corner>> = Mutex::new(None);

/// `sidebar.cornerOpen.visualize` paints the strips; otherwise they are all but
/// transparent — not wholly, which would let the pointer through as well.
const VISIBLE: u8 = 153;
const INVISIBLE: u8 = 1;
/// ponytail: the default accent, until the native surfaces follow the theme.
const ACCENT: COLORREF = COLORREF(0x00ff_840a);
/// Defined with the common controls, which the shell does not otherwise need.
const WM_MOUSELEAVE: u32 = 0x02A3;

/// The window, made the first time it is asked for. Main thread only.
fn window(app: &AppHandle) -> Option<HWND> {
    let existing = WINDOW.load(Ordering::Relaxed);
    if existing != 0 {
        return Some(HWND(existing as _));
    }
    let _ = APP.set(app.clone());
    unsafe {
        let instance = GetModuleHandleW(None).ok()?;
        let class = WNDCLASSW {
            lpfnWndProc: Some(procedure),
            hInstance: instance.into(),
            hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
            lpszClassName: w!("bw-hot-corners"),
            ..Default::default()
        };
        RegisterClassW(&class);
        let hwnd = CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_TOPMOST,
            w!("bw-hot-corners"),
            w!("beautiful-wallpaper — hotCorners"),
            WS_POPUP,
            0,
            0,
            GetSystemMetrics(SM_CXSCREEN),
            GetSystemMetrics(SM_CYSCREEN),
            None,
            None,
            instance,
            None,
        )
        .ok()?;
        WINDOW.store(hwnd.0 as isize, Ordering::Relaxed);
        Some(hwnd)
    }
}

/// Cuts the window down to these strips, and paints them or not.
pub fn shape(app: &AppHandle, corners: Vec<HotCorner>, visualize: bool) -> Result<HWND, String> {
    let hwnd = window(app).ok_or("could not make the hot corners' window")?;
    let rects: Vec<bw_core::capture::Rect> = corners.iter().map(|corner| corner.rect).collect();
    super::region::set_window_region(hwnd, &rects)?;
    *CORNERS.lock() = corners;
    unsafe {
        let alpha = if visualize { VISIBLE } else { INVISIBLE };
        SetLayeredWindowAttributes(hwnd, COLORREF(0), alpha, LWA_ALPHA)
            .map_err(|error| error.to_string())?;
        let _ = InvalidateRect(hwnd, None, true);
    }
    Ok(hwnd)
}

pub fn show(app: &AppHandle, visible: bool) {
    if let Some(hwnd) = window(app) {
        unsafe {
            let _ = ShowWindow(hwnd, if visible { SW_SHOWNOACTIVATE } else { SW_HIDE });
        }
    }
}

fn corner_at(x: i32, y: i32) -> Option<Corner> {
    CORNERS
        .lock()
        .iter()
        .find(|hot| {
            let r = hot.rect;
            x >= r.x && x < r.x + r.width && y >= r.y && y < r.y + r.height
        })
        .map(|hot| hot.corner)
}

/// Runs a corner's action, as the page's command did. Later, and not inside
/// the window procedure: opening a surface can mean building a window.
fn fire(app: &AppHandle, corner: Corner) {
    let each = app.clone();
    crate::surfaces::later(app, move || {
        if let Err(error) =
            crate::commands::run_hot_corner(each.clone(), each.state::<AppState>(), corner)
        {
            tracing::warn!(%error, "the hot corner did nothing");
        }
    });
}

unsafe extern "system" fn procedure(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let Some(app) = APP.get() else {
        return DefWindowProcW(hwnd, message, wparam, lparam);
    };
    // The window sits at the screen's origin, so client and screen agree.
    let x = (lparam.0 & 0xffff) as i16 as i32;
    let y = ((lparam.0 >> 16) & 0xffff) as i16 as i32;
    let config = || app.state::<AppState>().config().sidebar.corner_open.clone();

    match message {
        WM_MOUSEACTIVATE => return LRESULT(MA_NOACTIVATE as isize),
        WM_MOUSEMOVE => {
            let here = corner_at(x, y);
            let mut inside = INSIDE.lock();
            if here.is_some() && *inside != here {
                *inside = here;
                drop(inside);
                let mut leave = TRACKMOUSEEVENT {
                    cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
                    dwFlags: TME_LEAVE,
                    hwndTrack: hwnd,
                    dwHoverTime: 0,
                };
                let _ = TrackMouseEvent(&mut leave);
                if let (Some(corner), true) = (here, config().clickless) {
                    fire(app, corner);
                }
            }
            return LRESULT(0);
        }
        WM_MOUSELEAVE => {
            *INSIDE.lock() = None;
            return LRESULT(0);
        }
        WM_LBUTTONDOWN => {
            if let (Some(corner), false) = (corner_at(x, y), config().clickless) {
                fire(app, corner);
            }
            return LRESULT(0);
        }
        WM_MOUSEWHEEL => {
            // Screen coordinates, which here are the same thing.
            if let (Some(corner), true) = (corner_at(x, y), config().value_scroll) {
                let up = ((wparam.0 >> 16) & 0xffff) as i16 > 0;
                let each = app.clone();
                crate::surfaces::later(app, move || {
                    let _ = crate::commands::scroll_hot_corner(
                        each.state(),
                        each.state(),
                        each.state(),
                        corner,
                        up,
                    );
                });
            }
            return LRESULT(0);
        }
        WM_PAINT => {
            let mut paint = PAINTSTRUCT::default();
            let dc = BeginPaint(hwnd, &mut paint);
            let brush = CreateSolidBrush(ACCENT);
            FillRect(dc, &paint.rcPaint, brush);
            let _ = DeleteObject(brush);
            let _ = EndPaint(hwnd, &paint);
            return LRESULT(0);
        }
        _ => {}
    }
    DefWindowProcW(hwnd, message, wparam, lparam)
}
