//! Dragging a file off the shelf and onto it, and showing one in Explorer.
//!
//! Receiving a drop is nearly free: the page takes it like any page would, and
//! WebView2 hands over the paths a web page never sees ([`accept_drops`]).
//! Giving one back is not: an application that accepts a file expects an OLE
//! drag carrying shell items, which is a different mechanism from anything a
//! web page can start.
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

/// Passes `on_drop` what the page said a drop was for, and the paths of the
/// files dropped on it.
///
/// Tauri's own drop handling never heard of a drop on the shell's windows:
/// every drag ended over a drop target that took it and did nothing, so the
/// shelf showed the no-entry cursor and stayed empty. The page takes the drop
/// instead and passes the files on with
/// `chrome.webview.postMessageWithAdditionalObjects`, which WebView2 delivers
/// here as `ICoreWebView2File`s — and those carry the path.
pub fn accept_drops(
    window: &tauri::WebviewWindow,
    on_drop: impl Fn(String, Vec<String>) + Send + 'static,
) {
    use webview2_com::Microsoft::Web::WebView2::Win32::{
        ICoreWebView2File, ICoreWebView2WebMessageReceivedEventArgs2,
    };
    use webview2_com::{take_pwstr, WebMessageReceivedEventHandler};
    use webview2_windows_core::{Interface, PWSTR};

    let _ = window.with_webview(move |webview| unsafe {
        let Ok(core) = webview.controller().CoreWebView2() else {
            return;
        };
        let handler = WebMessageReceivedEventHandler::create(Box::new(move |_, args| {
            // Tauri's own messages carry no objects and fail this cast.
            let Some(args) = args else {
                return Ok(());
            };
            let Ok(with_objects) = args.cast::<ICoreWebView2WebMessageReceivedEventArgs2>() else {
                return Ok(());
            };
            let objects = with_objects.AdditionalObjects()?;
            let mut count = 0;
            objects.Count(&mut count)?;
            let mut paths = Vec::new();
            for index in 0..count {
                if let Ok(file) = objects.GetValueAtIndex(index)?.cast::<ICoreWebView2File>() {
                    let mut path = PWSTR::null();
                    file.Path(&mut path)?;
                    paths.push(take_pwstr(path));
                }
            }
            if !paths.is_empty() {
                let mut text = PWSTR::null();
                let message = match args.TryGetWebMessageAsString(&mut text) {
                    Ok(()) => take_pwstr(text),
                    Err(_) => String::new(),
                };
                // Dressed as a Tauri call so Tauri's handler, which reads it
                // first, parses it quietly (`lib/dropped.ts`); the purpose is
                // its `cmd`.
                let purpose = serde_json::from_str::<serde_json::Value>(&message)
                    .ok()
                    .and_then(|call| call.get("cmd")?.as_str().map(str::to_owned))
                    .unwrap_or_default();
                on_drop(purpose, paths);
            }
            Ok(())
        }));
        let mut token = 0;
        let _ = core.add_WebMessageReceived(&handler, &mut token);
    });
}

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
