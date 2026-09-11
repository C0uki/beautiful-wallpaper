//! Reading the Windows notification area.
//!
//! Wayland has StatusNotifierItem: applications publish their tray icons over
//! DBus and any shell can host them. Windows has nothing equivalent — icons are
//! owned by Explorer's own toolbar and there is no API to enumerate them.
//!
//! The approach every third-party Windows bar ends up at is to read Explorer's
//! toolbar across the process boundary: allocate a buffer inside Explorer,
//! ask its toolbar to fill in a `TBBUTTON` there, and read it back. It is
//! unsupported and undocumented, so everything here degrades to "no icons"
//! rather than failing, and clicking an icon is forwarded to its owner window
//! rather than synthesised inside Explorer.
//!
//! This is the least verifiable code in the shell: it cannot be exercised
//! without a real Explorer to read from. Treat a change here as needing a
//! manual check on Windows.

use serde::Serialize;
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{CloseHandle, HANDLE, HWND, LPARAM, WPARAM};
use windows::Win32::System::Diagnostics::Debug::ReadProcessMemory;
use windows::Win32::System::Memory::{
    VirtualAllocEx, VirtualFreeEx, MEM_COMMIT, MEM_RELEASE, MEM_RESERVE, PAGE_READWRITE,
};
use windows::Win32::System::Threading::{
    OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_OPERATION, PROCESS_VM_READ, PROCESS_VM_WRITE,
};
use windows::Win32::UI::Controls::{TBBUTTON, TBSTATE_HIDDEN};
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowExW, FindWindowW, GetWindowThreadProcessId, IsWindow, PostMessageW, SendMessageW,
    SetForegroundWindow, HICON, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_RBUTTONDOWN, WM_RBUTTONUP,
};

/// One icon in the notification area.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrayIcon {
    /// The owning window, as a string so it can round-trip through JSON.
    pub window: String,
    /// The id the owner registered the icon under.
    pub id: u32,
    pub tooltip: String,
    /// Whether Explorer currently hides this icon in the overflow flyout.
    pub hidden: bool,
    /// Cached PNG path for the icon, or empty — the same shape `WindowInfo`
    /// uses, so the bar draws it the way the dock draws an application.
    pub icon: String,
    /// The message the owner asked to be notified through. Opaque to the bar,
    /// which only hands it back to [`click`]; it travels with the icon because
    /// it is Explorer's record of the registration, not something we can look
    /// up again from the outside.
    pub callback_message: u32,
}

// Toolbar messages. `windows` exposes these as plain constants of the wrong
// integer type for `SendMessageW`, so they are restated here.
const TB_BUTTONCOUNT: u32 = 0x0400 + 24;
const TB_GETBUTTON: u32 = 0x0400 + 23;

/// Explorer's tray toolbar stores this behind each button's `dwData`.
///
/// Undocumented, and the layout differs between 32- and 64-bit Explorer; only
/// the 64-bit shape is handled, which is every supported Windows 10 and 11
/// installation.
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct TrayData {
    window: isize,
    id: u32,
    callback_message: u32,
    _reserved: [u32; 2],
    icon: isize,
}

/// Enumerates the notification area, visible icons first.
///
/// Returns an empty list rather than an error: a shell whose bar disappears
/// because Explorer restarted is worse than one that shows no icons for a moment.
pub fn icons() -> Vec<TrayIcon> {
    let mut found = Vec::new();
    unsafe {
        for (toolbar, hidden) in [(visible_toolbar(), false), (overflow_toolbar(), true)]
            .into_iter()
            .filter_map(|(toolbar, hidden)| toolbar.map(|toolbar| (toolbar, hidden)))
        {
            found.extend(read_toolbar(toolbar, hidden));
        }
    }
    found
}

/// `Shell_TrayWnd > TrayNotifyWnd > SysPager > ToolbarWindow32`.
///
/// Windows 11 dropped the `SysPager` level, so both shapes are tried.
unsafe fn visible_toolbar() -> Option<HWND> {
    let tray = FindWindowW(w!("Shell_TrayWnd"), PCWSTR::null()).ok()?;
    let notify = FindWindowExW(tray, HWND::default(), w!("TrayNotifyWnd"), PCWSTR::null()).ok()?;

    if let Ok(pager) = FindWindowExW(notify, HWND::default(), w!("SysPager"), PCWSTR::null()) {
        if let Ok(toolbar) = FindWindowExW(
            pager,
            HWND::default(),
            w!("ToolbarWindow32"),
            PCWSTR::null(),
        ) {
            return Some(toolbar);
        }
    }
    FindWindowExW(
        notify,
        HWND::default(),
        w!("ToolbarWindow32"),
        PCWSTR::null(),
    )
    .ok()
}

/// The flyout holding icons the user has chosen to hide.
unsafe fn overflow_toolbar() -> Option<HWND> {
    let overflow = FindWindowW(w!("NotifyIconOverflowWindow"), PCWSTR::null()).ok()?;
    FindWindowExW(
        overflow,
        HWND::default(),
        w!("ToolbarWindow32"),
        PCWSTR::null(),
    )
    .ok()
}

/// Reads every button out of one toolbar.
unsafe fn read_toolbar(toolbar: HWND, hidden_toolbar: bool) -> Vec<TrayIcon> {
    let count = SendMessageW(toolbar, TB_BUTTONCOUNT, WPARAM(0), LPARAM(0)).0;
    if count <= 0 {
        return Vec::new();
    }

    let Some(process) = RemoteProcess::open(toolbar) else {
        return Vec::new();
    };
    let Some(buffer) = process.allocate(std::mem::size_of::<TBBUTTON>()) else {
        return Vec::new();
    };

    let mut icons = Vec::new();
    for index in 0..count {
        // The toolbar writes the button into Explorer's own address space, so
        // the buffer handed to it has to live there too.
        let written = SendMessageW(
            toolbar,
            TB_GETBUTTON,
            WPARAM(index as usize),
            LPARAM(buffer.address as isize),
        );
        if written.0 == 0 {
            continue;
        }

        let Some(button) = process.read::<TBBUTTON>(buffer.address) else {
            continue;
        };
        if button.dwData == 0 {
            continue;
        }

        let Some(data) = process.read::<TrayData>(button.dwData as *const std::ffi::c_void) else {
            continue;
        };

        let window = HWND(data.window as *mut std::ffi::c_void);
        // Icons outlive their owners briefly when an application exits; skipping
        // them keeps dead entries out of the bar.
        if !IsWindow(window).as_bool() {
            continue;
        }

        // An icon we cannot rasterise is still an icon that is there: the bar
        // falls back to a dot rather than dropping the entry, which keeps the
        // degradation the same shape as everything else in this file.
        let icon = crate::platform::appicon::for_hicon(
            HICON(data.icon as *mut std::ffi::c_void),
            &format!("tray:{:#x}:{}", data.window, data.id),
        )
        .unwrap_or_default();

        icons.push(TrayIcon {
            window: format!("{:#x}", data.window),
            id: data.id,
            tooltip: String::new(),
            hidden: hidden_toolbar || button.fsState & TBSTATE_HIDDEN as u8 != 0,
            icon,
            callback_message: data.callback_message,
        });
    }

    icons
}

/// Forwards a click to the window that registered the icon.
///
/// `Shell_NotifyIcon`'s contract is that the owner is sent its chosen message
/// with the icon's id in `wParam` and the mouse message in `lParam`, so that is
/// what goes back — the click is not synthesised inside Explorer, which owns
/// nothing but the button we read it from.
///
/// The owner is brought to the foreground first. A context menu belongs to the
/// foreground window; raised from a background one it stays up after the next
/// click lands elsewhere, which is the oldest bug in the notification area and
/// the reason `SetForegroundWindow` is in every sample Microsoft ever shipped
/// for this.
///
/// ponytail: the classic registration only. An owner that asked for
/// `NOTIFYICON_VERSION_4` packs the cursor position into `wParam` and the
/// message and id into `lParam` instead, and will ignore this. Explorer's
/// button data does not record which version was asked for, so telling them
/// apart means probing — worth doing only if an application turns out not to
/// respond.
pub fn click(window: isize, id: u32, callback_message: u32, secondary: bool) {
    let owner = HWND(window as *mut std::ffi::c_void);
    let (down, up) = if secondary {
        (WM_RBUTTONDOWN, WM_RBUTTONUP)
    } else {
        (WM_LBUTTONDOWN, WM_LBUTTONUP)
    };

    unsafe {
        // The icon can go away between the bar drawing it and the user
        // reaching it; posting to a dead window is how this crashes.
        if !IsWindow(owner).as_bool() {
            return;
        }
        let _ = SetForegroundWindow(owner);

        // Both halves, because owners differ on which one they act upon.
        for message in [down, up] {
            let _ = PostMessageW(
                owner,
                callback_message,
                WPARAM(id as usize),
                LPARAM(message as isize),
            );
        }
    }
}

/// A handle to the process owning a window, plus the memory allocated in it.
struct RemoteProcess {
    handle: HANDLE,
}

struct RemoteBuffer<'a> {
    process: &'a RemoteProcess,
    address: *mut std::ffi::c_void,
}

impl RemoteProcess {
    unsafe fn open(window: HWND) -> Option<Self> {
        let mut pid = 0u32;
        GetWindowThreadProcessId(window, Some(&mut pid));
        if pid == 0 {
            return None;
        }

        let handle = OpenProcess(
            PROCESS_QUERY_INFORMATION | PROCESS_VM_OPERATION | PROCESS_VM_READ | PROCESS_VM_WRITE,
            false,
            pid,
        )
        .ok()?;

        Some(Self { handle })
    }

    unsafe fn allocate(&self, size: usize) -> Option<RemoteBuffer<'_>> {
        let address = VirtualAllocEx(
            self.handle,
            None,
            size,
            MEM_COMMIT | MEM_RESERVE,
            PAGE_READWRITE,
        );
        if address.is_null() {
            return None;
        }
        Some(RemoteBuffer {
            process: self,
            address,
        })
    }

    unsafe fn read<T: Copy + Default>(&self, address: *const std::ffi::c_void) -> Option<T> {
        let mut value = T::default();
        let mut read = 0usize;
        ReadProcessMemory(
            self.handle,
            address,
            std::ptr::addr_of_mut!(value).cast(),
            std::mem::size_of::<T>(),
            Some(&mut read),
        )
        .ok()?;

        // A short read means the layout assumption above is wrong for this
        // Explorer; better to report nothing than to act on half a struct.
        (read == std::mem::size_of::<T>()).then_some(value)
    }
}

impl Drop for RemoteProcess {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.handle);
        }
    }
}

impl Drop for RemoteBuffer<'_> {
    fn drop(&mut self) {
        unsafe {
            let _ = VirtualFreeEx(self.process.handle, self.address, 0, MEM_RELEASE);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tray_data_layout_matches_explorers_64_bit_shape() {
        // If this ever changes, every icon silently disappears — so the
        // assumption is pinned rather than left implicit.
        assert_eq!(std::mem::size_of::<TrayData>(), 32);
        assert_eq!(std::mem::align_of::<TrayData>(), 8);
    }

    #[test]
    fn toolbar_message_numbers_match_commctrl() {
        assert_eq!(TB_BUTTONCOUNT, 0x0418);
        assert_eq!(TB_GETBUTTON, 0x0417);
    }
}
