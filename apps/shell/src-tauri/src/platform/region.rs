//! Cutting a window down to the parts that should exist.
//!
//! Quickshell's `mask: Region` lets a panel cover the whole screen while only
//! a small patch of it accepts the pointer. Windows has no separate input
//! region, but it has something that serves: `SetWindowRgn` redefines the
//! window itself, and everything outside the region is neither drawn nor
//! hit-tested — a click there lands on whatever is underneath.
//!
//! That is what makes the hot corners possible without four separate windows.
//! One window covers the screen; its region is the union of the corner strips;
//! the rest of the desktop carries on as if the window were not there.
//!
//! The ownership rule is the trap. **On success the system takes the region
//! handle**, and deleting it afterwards frees memory the window manager is
//! still using. On failure the caller still owns it and must delete it. The
//! two paths are different and both have to be right.

use bw_core::capture::Rect;
use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Gdi::{
    CombineRgn, CreateRectRgn, DeleteObject, SetWindowRgn, HRGN, RGN_OR,
};

/// Restricts a window to these rectangles, in client coordinates.
///
/// An empty list clears the restriction and gives the whole window back, which
/// is what `SetWindowRgn` with a null region means.
pub fn set_window_region(hwnd: HWND, rects: &[Rect]) -> Result<(), String> {
    unsafe {
        if rects.is_empty() {
            // Null hands the window back whole; there is nothing to own.
            if SetWindowRgn(hwnd, None, true) == 0 {
                return Err("Windows would not clear the window's region".to_owned());
            }
            return Ok(());
        }

        let Some(combined) = union_of(rects) else {
            return Err("none of those rectangles has any area".to_owned());
        };

        if SetWindowRgn(hwnd, combined, true) == 0 {
            // Still ours, because the call did not take it.
            let _ = DeleteObject(combined);
            return Err("Windows would not shape the window".to_owned());
        }
        // Deliberately not deleted: the system owns it now, and freeing it
        // here would hand the window manager a dangling region.
        Ok(())
    }
}

/// One region covering all of them, or nothing if they are all empty.
unsafe fn union_of(rects: &[Rect]) -> Option<HRGN> {
    let mut combined: Option<HRGN> = None;

    for rect in rects {
        if rect.width <= 0 || rect.height <= 0 {
            continue;
        }
        // Right and bottom are exclusive, as everywhere else in GDI.
        let piece = CreateRectRgn(rect.x, rect.y, rect.x + rect.width, rect.y + rect.height);
        if piece.is_invalid() {
            continue;
        }

        match combined {
            None => combined = Some(piece),
            Some(target) => {
                // Combining into `target` overwrites it in place, so the piece
                // has served its purpose either way.
                CombineRgn(target, target, piece, RGN_OR);
                let _ = DeleteObject(piece);
            }
        }
    }

    combined
}
