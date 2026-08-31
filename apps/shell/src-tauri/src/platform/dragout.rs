//! Dragging a file off the shelf, and showing one in Explorer.
//!
//! Receiving a drop is free — the webview already has a shell drop target and
//! Tauri hands the paths over. Giving one back is not: an application that
//! accepts a file expects an OLE drag carrying shell items, which is a
//! different mechanism from anything a web page can start.
//!
//! Two shell functions do the work that would otherwise be two COM interfaces
//! implemented by hand. `SHCreateDataObject` builds the data object from item
//! id lists, and `SHDoDragDrop` supplies the default drop source — the one
//! that draws the drag image and the little plus sign, so the drag looks the
//! way every other drag on the machine looks.
//!
//! The drag is **modal**: `SHDoDragDrop` does not return until the button
//! comes up, pumping messages itself in the meantime. It therefore has to run
//! on the thread that owns the window and has OLE initialised, which is the
//! thread Tauri runs a synchronous command on. Making the command `async`
//! would move it to the runtime's pool and it would fail there.

use std::os::windows::ffi::OsStrExt;

use windows::core::PCWSTR;
use windows::Win32::Foundation::HWND;
use windows::Win32::System::Com::IDataObject;
use windows::Win32::System::Ole::{DROPEFFECT_COPY, DROPEFFECT_LINK, DROPEFFECT_NONE};
use windows::Win32::UI::Shell::Common::ITEMIDLIST;
use windows::Win32::UI::Shell::{
    ILCreateFromPathW, ILFree, SHCreateDataObject, SHDoDragDrop, SHOpenFolderAndSelectItems,
};

/// Item id lists that free themselves.
///
/// `SHCreateDataObject` copies what it is given, so these have to be released
/// afterwards — and the function between here and there can fail, so a guard
/// is the only way to be sure they are.
struct Pidls(Vec<*const ITEMIDLIST>);

impl Drop for Pidls {
    fn drop(&mut self) {
        for pidl in self.0.drain(..) {
            unsafe { ILFree(Some(pidl)) };
        }
    }
}

/// Turns paths into absolute item id lists.
///
/// A path that does not resolve is skipped rather than failing the drag: the
/// shelf can hold an entry whose file has since been moved, and dragging the
/// other four out of five is better than dragging none.
fn resolve(paths: &[String]) -> Pidls {
    Pidls(
        paths
            .iter()
            .filter_map(|path| {
                let wide = wide(path);
                let pidl = unsafe { ILCreateFromPathW(PCWSTR(wide.as_ptr())) };
                (!pidl.is_null()).then_some(pidl.cast_const())
            })
            .collect(),
    )
}

/// Starts a drag carrying these files. Returns whether anything was dropped.
///
/// A cancelled drag is not a failure — letting go over nothing is how someone
/// changes their mind — so it comes back as `false` rather than an error, and
/// the caller leaves the shelf alone.
pub fn drag_out(hwnd: HWND, paths: &[String]) -> Result<bool, String> {
    let pidls = resolve(paths);
    if pidls.0.is_empty() {
        return Err("none of those files are still where the shelf left them".to_owned());
    }

    unsafe {
        // No folder, so the item id lists are absolute — which is what
        // `ILCreateFromPathW` produces.
        let data: IDataObject = SHCreateDataObject(None, Some(&pidls.0), None)
            .map_err(|error| format!("could not describe those files to Windows: {error}"))?;

        // Copy and link only. A move would let the target delete the original,
        // which is not what putting something on a shelf asked for.
        let effect = SHDoDragDrop(hwnd, &data, None, DROPEFFECT_COPY | DROPEFFECT_LINK)
            .map_err(|error| format!("the drag was refused: {error}"))?;

        Ok(effect != DROPEFFECT_NONE)
    }
}

/// Opens the containing folder with the file selected.
///
/// Not `explorer /select,` in a new process: this reuses a window that is
/// already open on that folder, which is what happens when Explorer does it to
/// itself.
pub fn reveal(path: &str) -> Result<(), String> {
    let wide = wide(path);
    let pidl = unsafe { ILCreateFromPathW(PCWSTR(wide.as_ptr())) };
    if pidl.is_null() {
        return Err(format!("`{path}` is not there any more"));
    }
    let guard = Pidls(vec![pidl.cast_const()]);

    // Passing the item itself with no children is the documented way to ask
    // for "open the parent and select this".
    unsafe { SHOpenFolderAndSelectItems(guard.0[0], None, 0) }
        .map_err(|error| format!("could not show `{path}` in Explorer: {error}"))
}

fn wide(value: &str) -> Vec<u16> {
    std::ffi::OsStr::new(value)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}
