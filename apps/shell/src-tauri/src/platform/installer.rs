//! What the installer asks of `bw.exe`: what a shell has to set up and clean
//! up that an application does not. `installer.nsh` only calls these.
//!
//! Uninstalling an ordinary program removes its files and is done. A shell has
//! reached out and changed the machine on the user's behalf, and every one of
//! those changes outlives it:
//!
//!   * the stock taskbar may be hidden, and once these files are gone there is
//!     nothing left that can show it again;
//!   * an entry in the Run key would try to start a program that no longer
//!     exists, at every login;
//!   * the App Paths entry that makes `bw` resolve from the Run dialog names a
//!     folder that is about to be deleted.
//!
//! So [`uninstalling`] undoes them, in that order, and the taskbar goes first
//! because it is the one whose failure leaves somebody with a desktop they
//! cannot use. It runs before the uninstaller removes the files, while there is
//! still a binary to run, and nothing in it stops the uninstall: one that stops
//! because a helper failed is worse than one that leaves something behind, and
//! `bw taskbar show` is in the documentation for exactly that case.
//!
//! The user's own files — the config, the saved presets, the thumbnail cache —
//! are asked about rather than assumed either way. Deleting somebody's presets
//! without asking is rude; leaving a folder behind silently is untidy. Only the
//! question is honest, and it is asked here rather than by the installer, whose
//! own translated strings came up empty: the dialog had buttons and no question.

use std::path::{Path, PathBuf};

use windows::core::HSTRING;
use windows::Win32::UI::WindowsAndMessaging::{
    MessageBoxW, IDYES, MB_DEFBUTTON2, MB_ICONQUESTION, MB_SETFOREGROUND, MB_TOPMOST, MB_YESNO,
};

/// The Run dialog and Task Manager's "Run new task" resolve a bare name
/// through App Paths. This is the whole reason `bw taskbar show` is a rescue
/// anybody can actually type: the install folder is not on PATH, and this needs
/// no change to a variable shared with every other program on the machine.
const APP_PATHS: &str = r"Software\Microsoft\Windows\CurrentVersion\App Paths\bw.exe";

/// The folders the user's files live in, under `%APPDATA%` and `%LOCALAPPDATA%`.
const FOLDER: &str = "beautiful-wallpaper";

/// After the files are in place: register `bw` with App Paths.
pub fn installed() -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|error| error.to_string())?;
    let folder = exe.parent().unwrap_or(&exe);
    let key = windows_registry::CURRENT_USER
        .create(APP_PATHS)
        .map_err(|error| error.to_string())?;
    key.set_string("", exe.to_string_lossy())
        .and_then(|()| key.set_string("Path", folder.to_string_lossy()))
        .map_err(|error| error.to_string())
}

/// Before the files go. `quiet` for an uninstall nobody is watching, which
/// keeps the user's files rather than asking.
pub fn uninstalling(quiet: bool) {
    unsafe {
        // A shell killed while hiding it also left it set to hide itself.
        super::win::restore_saved_taskbar_state();
        super::win::restore_taskbar_places();
        super::win::set_taskbar_visible(true);
    }
    if let Err(error) = super::autostart::apply(false) {
        eprintln!("{error}");
    }
    let _ = windows_registry::CURRENT_USER.remove_tree(APP_PATHS);

    if !quiet && asked_to_remove_data() {
        remove_data();
    }
}

/// Asked, not assumed. "No" is the default so that an uninstall somebody
/// clicked through in a hurry keeps their wallpapers and presets.
fn asked_to_remove_data() -> bool {
    let config = bw_core::config::load(&bw_core::paths::config_file()).unwrap_or_default();
    let language = match config.language.ui.as_str() {
        "auto" => crate::services::weather::system_language(),
        chosen => chosen.to_owned(),
    };
    let question = if language.starts_with("ja") {
        "beautiful-wallpaper の設定・プリセット・サムネイルキャッシュも削除しますか?\n\n「いいえ」を選ぶと、次にインストールするときのために残しておけます。"
    } else {
        "Remove your beautiful-wallpaper settings, presets and cached thumbnails as well?\n\nChoose No to keep them for a future installation."
    };
    // On top and in front: the uninstaller's own window is the foreground one,
    // and a question behind it would look like an uninstall that hung.
    let answer = unsafe {
        MessageBoxW(
            None,
            &HSTRING::from(question),
            &HSTRING::from("beautiful-wallpaper"),
            MB_YESNO | MB_ICONQUESTION | MB_DEFBUTTON2 | MB_SETFOREGROUND | MB_TOPMOST,
        )
    };
    answer == IDYES
}

/// The user's folders, gone. The one under `%LOCALAPPDATA%` is also where the
/// shell is installed by default: there everything but this program and the
/// uninstaller goes now, and the uninstaller takes those and the folder after.
fn remove_data() {
    let own = std::env::current_exe().ok();
    for root in ["APPDATA", "LOCALAPPDATA"] {
        let Some(base) = std::env::var_os(root) else {
            continue;
        };
        let folder = PathBuf::from(base).join(FOLDER);
        let holds_us = own
            .as_deref()
            .and_then(Path::parent)
            .is_some_and(|parent| same(parent, &folder));
        if !holds_us {
            let _ = std::fs::remove_dir_all(&folder);
            continue;
        }
        let Ok(entries) = std::fs::read_dir(&folder) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if spared(&path, own.as_deref()) {
                continue;
            }
            let _ = if path.is_dir() {
                std::fs::remove_dir_all(&path)
            } else {
                std::fs::remove_file(&path)
            };
        }
    }
}

/// This program, and the uninstaller that is running it.
fn spared(path: &Path, own: Option<&Path>) -> bool {
    own.is_some_and(|own| same(path, own))
        || path
            .file_name()
            .is_some_and(|name| name.eq_ignore_ascii_case("uninstall.exe"))
}

/// Whether two paths name the same place, as Windows compares them: without
/// regard to case.
fn same(a: &Path, b: &Path) -> bool {
    a.as_os_str().eq_ignore_ascii_case(b.as_os_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_uninstaller_and_this_program_are_spared_and_nothing_else() {
        let own = Path::new(r"C:\Users\me\AppData\Local\beautiful-wallpaper\bw.exe");
        let folder = Path::new(r"C:\Users\me\AppData\Local\beautiful-wallpaper");
        assert!(spared(own, Some(own)));
        assert!(spared(&folder.join("Uninstall.exe"), Some(own)));
        assert!(spared(
            Path::new(r"c:\users\me\appdata\local\beautiful-wallpaper\BW.EXE"),
            Some(own)
        ));
        assert!(!spared(&folder.join("config.json"), Some(own)));
        assert!(!spared(&folder.join("cache"), Some(own)));
        assert!(!spared(&folder.join("cache"), None));
    }
}
