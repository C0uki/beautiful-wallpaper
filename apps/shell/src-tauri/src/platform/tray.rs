//! The notification area, taken at the source.
//!
//! Wayland has StatusNotifierItem: applications publish their tray icons over
//! DBus and any shell can host them. Windows has nothing so polite. An
//! application's `Shell_NotifyIcon` finds the first top-level window of class
//! `Shell_TrayWnd` and sends it the icon in a `WM_COPYDATA`. Explorer used to
//! keep those in a toolbar another process could read; the Windows 11 taskbar
//! keeps them nowhere anyone else can reach, so reading Explorer's copy gave
//! an empty tray.
//!
//! So the shell makes a `Shell_TrayWnd` of its own, above Explorer's, and is
//! sent every icon first. It keeps what it learns and passes every message on
//! to Explorer, so Explorer's copy stays whole — the icons are still
//! there when the shell quits — and everything else that talks to the taskbar
//! through that window (app bars, `WM_COMMAND`s, Explorer's own private
//! messages) still reaches it. Icons registered before the shell started are
//! asked for again with `TaskbarCreated`, which every application answers by
//! adding its icons anew.
//!
//! What being first gives over reading Explorer's copy: the tooltip, and the
//! version each icon asked for, which decides how a click has to be packed.

use std::sync::atomic::{AtomicIsize, Ordering};
use std::sync::OnceLock;

use parking_lot::Mutex;
use serde::Serialize;
use windows::core::{w, GUID, PCWSTR};
use windows::Win32::Foundation::{HANDLE, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::System::DataExchange::COPYDATASTRUCT;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Shell::{SHAllocShared, SHFreeShared, SHLockShared, SHUnlockShared};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetCursorPos, GetMessageW, GetWindowRect,
    GetWindowThreadProcessId, InSendMessage, IsWindow, KillTimer, PostMessageW, RegisterClassExW,
    SendMessageTimeoutW, SetForegroundWindow, SetTimer, SetWindowPos, HICON, HWND_TOPMOST, MSG,
    SMTO_ABORTIFHUNG, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, WINDOW_EX_STYLE, WM_COMMAND,
    WM_CONTEXTMENU, WM_COPYDATA, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_RBUTTONDOWN, WM_RBUTTONUP,
    WM_SYSCOMMAND, WM_TIMER, WM_USER, WNDCLASSEXW, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};

use crate::platform::win;

/// One icon in the notification area.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrayIcon {
    /// The owning window, as a string so it can round-trip through JSON.
    pub window: String,
    /// The id the owner registered the icon under.
    pub id: u32,
    pub tooltip: String,
    /// Whether it belongs in the overflow rather than on the bar: hidden by
    /// its owner, or not one the person chose to keep on the taskbar.
    pub hidden: bool,
    /// Cached PNG path for the icon, or empty — the same shape `WindowInfo`
    /// uses, so the bar draws it the way the dock draws an application.
    pub icon: String,
    /// The message the owner asked to be notified through. Opaque to the bar,
    /// which only hands it back to [`click`].
    pub callback_message: u32,
}

/// What the shell has been told about one icon.
struct Hosted {
    window: isize,
    id: u32,
    /// Set when the icon is known by a GUID rather than by window and id.
    guid: Option<GUID>,
    callback: u32,
    /// `NOTIFYICON_VERSION_4` or older; see [`click`].
    version: u32,
    tooltip: String,
    hidden_by_owner: bool,
    promoted: bool,
    image: String,
}

static ICONS: Mutex<Vec<Hosted>> = Mutex::new(Vec::new());
static HOST: AtomicIsize = AtomicIsize::new(0);
static ON_CHANGE: OnceLock<Box<dyn Fn() + Send + Sync>> = OnceLock::new();

/// What `Shell_NotifyIcon` sends: `SHELLTRAYDATA` around a
/// `NOTIFYICONDATAW` whose handles are 32 bits wide in every process, so a
/// 32- and a 64-bit application send the same bytes.
#[repr(C)]
#[derive(Clone, Copy)]
struct TrayMessage {
    signature: u32,
    message: u32,
    data: NotifyIconData32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct NotifyIconData32 {
    size: u32,
    window: u32,
    id: u32,
    flags: u32,
    callback: u32,
    icon: u32,
    tip: [u16; 128],
    state: u32,
    state_mask: u32,
    info: [u16; 256],
    /// `uTimeout` or `uVersion`: a union in the original.
    version: u32,
    info_title: [u16; 64],
    info_flags: u32,
    guid: GUID,
    balloon_icon: u32,
}

const SIGNATURE: u32 = 0x3475_3423;
/// `COPYDATASTRUCT::dwData` for a notification-icon message.
const TRAY_DATA: usize = 1;
/// `COPYDATASTRUCT::dwData` for an app bar's `SHAppBarMessage`.
const APP_BAR_DATA: usize = 0;

const NIM_ADD: u32 = 0;
const NIM_MODIFY: u32 = 1;
const NIM_DELETE: u32 = 2;
const NIM_SETVERSION: u32 = 4;
const NIF_MESSAGE: u32 = 0x01;
const NIF_ICON: u32 = 0x02;
const NIF_TIP: u32 = 0x04;
const NIF_STATE: u32 = 0x08;
const NIF_GUID: u32 = 0x20;
const NIS_HIDDEN: u32 = 0x01;
const NIN_SELECT: u32 = WM_USER;

/// Timers on the host window.
const KEEP_ON_TOP: usize = 1;
const NOTIFY: usize = 2;
const ASK_AGAIN: usize = 3;

/// Starts hosting, calling `on_change` whenever the icons change.
pub fn host(on_change: impl Fn() + Send + Sync + 'static) {
    if ON_CHANGE.set(Box::new(on_change)).is_err() {
        return;
    }
    let _ = std::thread::Builder::new()
        .name("bw-tray-host".to_owned())
        .spawn(|| unsafe { run() });
    watch_promotions();
}

/// Reads every icon's [`promoted`] again whenever Settings changes which
/// icons are kept on the taskbar, so a switch flipped there shows at once
/// rather than the next time the application adds its icon.
fn watch_promotions() {
    use windows::Win32::Foundation::{BOOL, HANDLE};
    use windows::Win32::System::Registry::{
        RegCloseKey, RegNotifyChangeKeyValue, RegOpenKeyExW, HKEY, HKEY_CURRENT_USER, KEY_NOTIFY,
        REG_NOTIFY_CHANGE_LAST_SET, REG_NOTIFY_CHANGE_NAME,
    };

    let _ = std::thread::Builder::new()
        .name("bw-tray-settings".to_owned())
        .spawn(|| unsafe {
            let mut key = HKEY::default();
            if RegOpenKeyExW(
                HKEY_CURRENT_USER,
                w!("Control Panel\\NotifyIconSettings"),
                0,
                KEY_NOTIFY,
                &mut key,
            )
            .is_err()
            {
                return;
            }
            // Blocks until something under the key changes, every time.
            while RegNotifyChangeKeyValue(
                key,
                BOOL::from(true),
                REG_NOTIFY_CHANGE_NAME | REG_NOTIFY_CHANGE_LAST_SET,
                HANDLE::default(),
                BOOL::from(false),
            )
            .is_ok()
            {
                // The registry is read with the icons let go of: the host
                // thread takes the same lock for every message it forwards.
                let asked: Vec<(isize, u32)> = ICONS
                    .lock()
                    .iter()
                    .map(|icon| (icon.window, icon.id))
                    .collect();
                let answers: Vec<bool> = asked
                    .iter()
                    .map(|&(window, id)| promoted(window, id))
                    .collect();
                let mut changed = false;
                for icon in ICONS.lock().iter_mut() {
                    let Some(at) = asked
                        .iter()
                        .position(|&(window, id)| window == icon.window && id == icon.id)
                    else {
                        continue;
                    };
                    changed |= icon.promoted != answers[at];
                    icon.promoted = answers[at];
                }
                if changed {
                    if let Some(on_change) = ON_CHANGE.get() {
                        on_change();
                    }
                }
            }
            let _ = RegCloseKey(key);
        });
}

/// The shell's own `Shell_TrayWnd`, once it exists.
pub fn host_window() -> HWND {
    HWND(HOST.load(Ordering::Relaxed) as _)
}

unsafe fn run() {
    let Ok(module) = GetModuleHandleW(PCWSTR::null()) else {
        return;
    };
    let instance: windows::Win32::Foundation::HINSTANCE = module.into();
    let class = WNDCLASSEXW {
        cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
        lpfnWndProc: Some(window_proc),
        hInstance: instance,
        lpszClassName: w!("Shell_TrayWnd"),
        ..Default::default()
    };
    if RegisterClassExW(&class) == 0 {
        tracing::warn!("could not register the notification area's window class");
        return;
    }
    // Never shown: it only has to be found, and found first, which a topmost
    // window is whether it is visible or not.
    let Ok(window) = CreateWindowExW(
        WINDOW_EX_STYLE(WS_EX_TOOLWINDOW.0 | WS_EX_TOPMOST.0),
        w!("Shell_TrayWnd"),
        PCWSTR::null(),
        WS_POPUP,
        0,
        0,
        0,
        0,
        None,
        None,
        instance,
        None,
    ) else {
        tracing::warn!("could not create the notification area's window");
        return;
    };
    HOST.store(window.0 as isize, Ordering::Relaxed);
    keep_on_top(window);
    // Explorer's taskbar raises itself as it is used; this puts the host back
    // in front of it before an application looks for one.
    SetTimer(window, KEEP_ON_TOP, 2000, None);
    // Everything already in the notification area, added again — to here.
    win::announce_taskbar();

    let mut message = MSG::default();
    while GetMessageW(&mut message, None, 0, 0).as_bool() {
        DispatchMessageW(&message);
    }
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if msg == WM_COPYDATA && lparam.0 != 0 {
        let copy = &*(lparam.0 as *const COPYDATASTRUCT);
        if copy.dwData == TRAY_DATA {
            record(copy);
            let _ = forward(msg, wparam, lparam);
            // Ours took it, whatever Explorer says: after `TaskbarCreated`
            // Explorer refuses an icon it already has, and an application
            // told its icon failed may try again or give up.
            return LRESULT(1);
        }
        if copy.dwData == APP_BAR_DATA {
            if let Some(result) = forward_own_app_bar(copy, wparam) {
                return result;
            }
        }
        return forward(msg, wparam, lparam);
    }

    if msg == WM_TIMER {
        match wparam.0 {
            KEEP_ON_TOP => keep_on_top(hwnd),
            NOTIFY => {
                let _ = KillTimer(hwnd, NOTIFY);
                if let Some(on_change) = ON_CHANGE.get() {
                    on_change();
                }
            }
            ASK_AGAIN => {
                let _ = KillTimer(hwnd, ASK_AGAIN);
                win::announce_taskbar();
            }
            _ => {}
        }
        return LRESULT(0);
    }

    if msg == win::taskbar_created() {
        // Explorer came back with a new taskbar of its own, made after this
        // one and so in front of it: everything re-added just now went
        // there. Get in front again and ask once more, when it has settled.
        if !win::announced_by_us() {
            keep_on_top(hwnd);
            SetTimer(hwnd, ASK_AGAIN, 2000, None);
        }
        return LRESULT(0);
    }

    // Anything else meant for the taskbar is Explorer's: its commands, its
    // own private messages, and whatever other programs ask of it. Not the
    // registered messages from 0xC000 up: those arrive as broadcasts, which
    // Explorer has already been sent once.
    if msg == WM_COMMAND || msg == WM_SYSCOMMAND || (WM_USER..0xC000).contains(&msg) {
        return forward(msg, wparam, lparam);
    }

    DefWindowProcW(hwnd, msg, wparam, lparam)
}

/// Passes a message on to Explorer's taskbar, the way it arrived.
/// The shell's own `SHAppBarMessage`, the bar reserving its edge. It finds
/// this host first like everyone's does, but seeing a window of its own
/// process it hands over the bar's position as memory only it can read,
/// where it would otherwise share it with the taskbar's process. Passed on
/// as it is, Explorer cannot read it and the call does nothing while
/// reporting success: no edge reserved. So the position is copied into
/// memory shared with Explorer, and Explorer's answer copied back.
unsafe fn forward_own_app_bar(copy: &COPYDATASTRUCT, wparam: WPARAM) -> Option<LRESULT> {
    // The 64-bit layout: the 40-byte `APPBARDATA` in its 32/64-bit-neutral
    // form, the message, then at 48 the shared memory and at 56 the
    // process it was made for.
    if copy.cbData != 64 {
        return None;
    }
    let mut block = [0u8; 64];
    std::ptr::copy_nonoverlapping(copy.lpData as *const u8, block.as_mut_ptr(), 64);
    let shared = u64::from_le_bytes(block[48..56].try_into().ok()?) as usize;
    let process = u32::from_le_bytes(block[56..60].try_into().ok()?);
    let ours = std::process::id();
    if shared == 0 || process != ours {
        return None;
    }
    let mut explorer = 0u32;
    GetWindowThreadProcessId(win::explorer_tray()?, Some(&mut explorer));
    let own = SHLockShared(HANDLE(shared as _), ours);
    if own.is_null() {
        return None;
    }
    let theirs = SHAllocShared(Some(own), 40, explorer);
    if theirs.is_invalid() {
        let _ = SHUnlockShared(own);
        return None;
    }
    block[48..56].copy_from_slice(&(theirs.0 as u64).to_le_bytes());
    block[56..60].copy_from_slice(&explorer.to_le_bytes());
    let repacked = COPYDATASTRUCT {
        dwData: APP_BAR_DATA,
        cbData: 64,
        lpData: block.as_mut_ptr().cast(),
    };
    let result = forward(
        WM_COPYDATA,
        wparam,
        LPARAM(std::ptr::addr_of!(repacked) as isize),
    );
    let answer = SHLockShared(theirs, explorer);
    if !answer.is_null() {
        std::ptr::copy_nonoverlapping(answer as *const u8, own as *mut u8, 40);
        let _ = SHUnlockShared(answer);
    }
    let _ = SHFreeShared(theirs, explorer);
    let _ = SHUnlockShared(own);
    Some(result)
}

unsafe fn forward(msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let Some(explorer) = win::explorer_tray() else {
        return LRESULT(0);
    };
    if !InSendMessage().as_bool() {
        let _ = PostMessageW(explorer, msg, wparam, lparam);
        return LRESULT(0);
    }
    let mut result = 0usize;
    let _ = SendMessageTimeoutW(
        explorer,
        msg,
        wparam,
        lparam,
        SMTO_ABORTIFHUNG,
        3000,
        Some(std::ptr::addr_of_mut!(result)),
    );
    LRESULT(result as isize)
}

/// Over Explorer's taskbar, at its size: the first `Shell_TrayWnd` found, and
/// in the place a program measuring "the taskbar" expects it.
unsafe fn keep_on_top(hwnd: HWND) {
    let mut bounds = RECT::default();
    match win::explorer_tray() {
        Some(explorer) if GetWindowRect(explorer, &mut bounds).is_ok() => {
            let _ = SetWindowPos(
                hwnd,
                HWND_TOPMOST,
                bounds.left,
                bounds.top,
                bounds.right - bounds.left,
                bounds.bottom - bounds.top,
                SWP_NOACTIVATE,
            );
        }
        _ => {
            let _ = SetWindowPos(
                hwnd,
                HWND_TOPMOST,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            );
        }
    }
}

/// Takes in one `Shell_NotifyIcon` call.
unsafe fn record(copy: &COPYDATASTRUCT) {
    let length = copy.cbData as usize;
    // Up to the tooltip at least; anything shorter is not a notify message.
    if copy.lpData.is_null() || length < 8 + 24 {
        return;
    }
    let mut message: TrayMessage = std::mem::zeroed();
    std::ptr::copy_nonoverlapping(
        copy.lpData.cast::<u8>(),
        std::ptr::addr_of_mut!(message).cast::<u8>(),
        length.min(std::mem::size_of::<TrayMessage>()),
    );
    if message.signature != SIGNATURE {
        return;
    }
    let data = message.data;
    // A handle sent as 32 bits is widened the way Windows widens it: signed.
    let window = data.window as i32 as isize;
    let guid = (data.flags & NIF_GUID != 0).then_some(data.guid);

    let mut icons = ICONS.lock();
    let found = icons.iter().position(|icon| match guid {
        Some(guid) => icon.guid == Some(guid),
        None => icon.guid.is_none() && icon.window == window && icon.id == data.id,
    });
    match message.message {
        NIM_DELETE => {
            if let Some(index) = found {
                icons.remove(index);
            }
        }
        NIM_SETVERSION => {
            if let Some(index) = found {
                icons[index].version = data.version;
            }
        }
        NIM_ADD | NIM_MODIFY => {
            let index = match found {
                Some(index) => index,
                None if message.message == NIM_ADD => {
                    icons.push(Hosted {
                        window,
                        id: data.id,
                        guid,
                        callback: 0,
                        version: 0,
                        tooltip: String::new(),
                        hidden_by_owner: false,
                        promoted: promoted(window, data.id),
                        image: String::new(),
                    });
                    icons.len() - 1
                }
                None => return,
            };
            let icon = &mut icons[index];
            icon.window = window;
            icon.id = data.id;
            if data.flags & NIF_MESSAGE != 0 {
                icon.callback = data.callback;
            }
            // Drawn now, while the owner is waiting on this call: the handle
            // is only certain to be alive until it returns.
            if data.flags & NIF_ICON != 0 {
                let handle = HICON(data.icon as i32 as isize as *mut std::ffi::c_void);
                icon.image = crate::platform::appicon::for_hicon(handle).unwrap_or_default();
            }
            if data.flags & NIF_TIP != 0 {
                let end = data
                    .tip
                    .iter()
                    .position(|&c| c == 0)
                    .unwrap_or(data.tip.len());
                icon.tooltip = String::from_utf16_lossy(&data.tip[..end]);
            }
            if data.flags & NIF_STATE != 0 && data.state_mask & NIS_HIDDEN != 0 {
                icon.hidden_by_owner = data.state & NIS_HIDDEN != 0;
            }
        }
        _ => return,
    }
    drop(icons);

    // A tenth of a second to gather a burst — an application adding an icon
    // sends three or four of these back to back — into one update.
    let _ = SetTimer(host_window(), NOTIFY, 100, None);
}

/// Whether the person keeps this icon on the taskbar: Settings >
/// Personalization > Taskbar, recorded per executable and id under
/// `NotifyIconSettings`. An icon Windows has not recorded yet is shown.
///
/// Read when the icon is added, and again by [`watch_promotions`] whenever
/// Settings changes.
fn promoted(window: isize, id: u32) -> bool {
    let mut process = 0u32;
    unsafe { GetWindowThreadProcessId(HWND(window as _), Some(&mut process)) };
    let Some(executable) = crate::platform::appicon::executable_for(process) else {
        return true;
    };
    let Ok(settings) = windows_registry::CURRENT_USER.open(r"Control Panel\NotifyIconSettings")
    else {
        return true;
    };
    let Ok(entries) = settings.keys() else {
        return true;
    };
    for name in entries {
        let Ok(entry) = settings.open(&name) else {
            continue;
        };
        let Ok(path) = entry.get_string("ExecutablePath") else {
            continue;
        };
        if !expand_known_folder(&path).eq_ignore_ascii_case(&executable) {
            continue;
        }
        if entry.get_u32("UID").is_ok_and(|uid| uid != id) {
            continue;
        }
        return entry.get_u32("IsPromoted").unwrap_or(0) != 0;
    }
    true
}

/// `{F38BF404-…}\explorer.exe` with the known folder spelled out.
fn expand_known_folder(path: &str) -> String {
    use windows::Win32::UI::Shell::{SHGetKnownFolderPath, KF_FLAG_DEFAULT};
    let Some((folder, rest)) = path.strip_prefix('{').and_then(|tail| tail.split_once('}')) else {
        return path.to_owned();
    };
    // Parsed by hand: `GUID::from` panics on text it cannot read, and this
    // text is whatever the registry holds.
    let digits: String = folder.chars().filter(|&c| c != '-').collect();
    let Some(folder) = (digits.len() == 32)
        .then(|| u128::from_str_radix(&digits, 16).ok())
        .flatten()
        .map(GUID::from_u128)
    else {
        return path.to_owned();
    };
    unsafe {
        let Ok(found) = SHGetKnownFolderPath(&folder, KF_FLAG_DEFAULT, None) else {
            return path.to_owned();
        };
        let expanded = found.to_string().unwrap_or_default();
        windows::Win32::System::Com::CoTaskMemFree(Some(found.0 as _));
        format!("{expanded}{rest}")
    }
}

/// Every icon the shell knows of, its dead ones let go.
pub fn icons() -> Vec<TrayIcon> {
    let mut icons = ICONS.lock();
    // An application that exits without removing its icon leaves it behind;
    // its window is the tell.
    icons.retain(|icon| unsafe { IsWindow(HWND(icon.window as _)).as_bool() });
    icons
        .iter()
        .map(|icon| TrayIcon {
            window: format!("{:#x}", icon.window),
            id: icon.id,
            tooltip: icon.tooltip.clone(),
            hidden: icon.hidden_by_owner || !icon.promoted,
            icon: icon.image.clone(),
            callback_message: icon.callback,
        })
        .collect()
}

/// Forwards a click to the window that registered the icon, packed the way
/// the version it asked for expects.
///
/// Older registrations get the icon's id in `wParam` and the mouse message in
/// `lParam`. `NOTIFYICON_VERSION_4` gets the cursor in `wParam` and the
/// message and id together in `lParam`, and like version 3 is told the click
/// completed — `NIN_SELECT` for the main button, `WM_CONTEXTMENU` for the
/// other — which is what most of them act on.
///
/// The owner may take the foreground for its menu: a context menu belongs to
/// the foreground window, and raised from a background one it stays up after
/// the next click lands elsewhere.
pub fn click(window: isize, id: u32, callback_message: u32, secondary: bool) {
    use windows::Win32::UI::WindowsAndMessaging::AllowSetForegroundWindow;

    let owner = HWND(window as *mut std::ffi::c_void);
    let version = ICONS
        .lock()
        .iter()
        .find(|icon| icon.window == window && icon.id == id)
        .map_or(0, |icon| icon.version);
    let (down, up, done) = if secondary {
        (WM_RBUTTONDOWN, WM_RBUTTONUP, WM_CONTEXTMENU)
    } else {
        (WM_LBUTTONDOWN, WM_LBUTTONUP, NIN_SELECT)
    };

    unsafe {
        // The icon can go away between the bar drawing it and the click.
        if !IsWindow(owner).as_bool() {
            return;
        }
        let mut process = 0u32;
        GetWindowThreadProcessId(owner, Some(&mut process));
        let _ = AllowSetForegroundWindow(process);
        let _ = SetForegroundWindow(owner);

        let mut cursor = POINT::default();
        let _ = GetCursorPos(&mut cursor);
        let at = (cursor.x as u16 as usize) | ((cursor.y as u16 as usize) << 16);
        let messages: &[u32] = if version >= 3 {
            &[down, up, done]
        } else {
            &[down, up]
        };
        for &message in messages {
            let (wparam, lparam) = if version >= 4 {
                (
                    at,
                    (message as usize & 0xffff) | ((id as usize & 0xffff) << 16),
                )
            } else {
                (id as usize, message as usize)
            };
            let _ = PostMessageW(
                owner,
                callback_message,
                WPARAM(wparam),
                LPARAM(lparam as isize),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_notify_data_matches_the_32_bit_layout_shell32_sends() {
        // `NOTIFYICONDATAW` with 32-bit handles, through `hBalloonIcon`: if
        // this drifts, every field after the drift is read from the wrong
        // place and icons come and go at random.
        assert_eq!(std::mem::size_of::<NotifyIconData32>(), 956);
        assert_eq!(std::mem::offset_of!(NotifyIconData32, version), 800);
        assert_eq!(std::mem::offset_of!(NotifyIconData32, guid), 936);
        assert_eq!(std::mem::offset_of!(TrayMessage, data), 8);
    }
}
