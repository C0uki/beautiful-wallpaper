//! Turning a list of windows into a list of dock icons.
//!
//! The original groups Wayland toplevels by `appId`, which the compositor
//! hands it. Windows has no such thing: a window knows its process, and a
//! process knows its executable, so the executable's path is what identifies
//! an application here. Two windows of the same program share a path even when
//! their titles have nothing in common, which is exactly the grouping a dock
//! wants.
//!
//! This is all pure data so that it is covered by tests that run on Linux; the
//! enumeration that produces the input is in the shell crate and cannot be.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// One window, as the platform layer reports it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct WindowInfo {
    /// The window handle, as a string. Rust has no stable integer type for an
    /// `HWND` that survives a JSON round trip intact, and the frontend only
    /// ever passes it back.
    pub id: String,
    pub title: String,
    /// Full path of the owning process's executable, lowercased for matching.
    pub executable: String,
    /// What to call the application.
    pub name: String,
    /// Cached PNG path for the executable's icon, or empty.
    pub icon: String,
    pub active: bool,
}

/// One icon on the dock: an application, with whatever windows it has open.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DockApp {
    pub executable: String,
    pub name: String,
    pub icon: String,
    pub windows: Vec<WindowInfo>,
    /// Kept on the dock whether or not it is running.
    pub pinned: bool,
    /// Whether one of its windows is the foreground window.
    pub active: bool,
}

/// Groups windows into dock icons, in the order the dock draws them.
///
/// Pinned applications come first, in the order the user pinned them, so the
/// dock does not reshuffle as programs start and stop. Everything else follows
/// in the order its first window was reported.
pub fn group(windows: &[WindowInfo], pinned: &[String], ignored: &[String]) -> Vec<DockApp> {
    let mut apps: Vec<DockApp> = Vec::new();

    // Pinned entries exist even with no window open — that is what pinning is.
    for path in pinned {
        let key = normalise(path);
        if apps.iter().any(|app| app.executable == key) {
            continue;
        }
        apps.push(DockApp {
            name: file_stem(&key),
            executable: key,
            icon: String::new(),
            windows: Vec::new(),
            pinned: true,
            active: false,
        });
    }

    for window in windows {
        let key = normalise(&window.executable);
        if is_ignored(&key, ignored) {
            continue;
        }

        match apps.iter_mut().find(|app| app.executable == key) {
            Some(app) => {
                // A pinned entry has no icon or real name until a window
                // arrives to supply them.
                if app.icon.is_empty() {
                    app.icon = window.icon.clone();
                }
                if !window.name.is_empty() {
                    app.name = window.name.clone();
                }
                app.active |= window.active;
                app.windows.push(window.clone());
            }
            None => apps.push(DockApp {
                executable: key,
                name: if window.name.is_empty() {
                    file_stem(&window.executable)
                } else {
                    window.name.clone()
                },
                icon: window.icon.clone(),
                active: window.active,
                windows: vec![window.clone()],
                pinned: false,
            }),
        }
    }

    apps
}

/// Whether an executable is on the ignore list.
fn is_ignored(executable: &str, ignored: &[String]) -> bool {
    let name = file_name(executable);
    ignored
        .iter()
        .any(|pattern| matches_glob(&pattern.to_lowercase(), &name))
}

/// A case-insensitive glob with `*` and `?`, matched against a file name.
///
/// Written out rather than pulled in: the portable crate has no regex engine,
/// and an ignore list of a few executable names does not justify one.
pub fn matches_glob(pattern: &str, name: &str) -> bool {
    let pattern: Vec<char> = pattern.chars().collect();
    let name: Vec<char> = name.chars().collect();

    // The classic two-pointer wildcard match: linear, no backtracking blowup
    // on a pattern like `*a*a*a*`.
    let (mut p, mut n) = (0usize, 0usize);
    let (mut star, mut resume) = (None, 0usize);

    while n < name.len() {
        if p < pattern.len() && (pattern[p] == '?' || pattern[p] == name[n]) {
            p += 1;
            n += 1;
        } else if p < pattern.len() && pattern[p] == '*' {
            star = Some(p);
            resume = n;
            p += 1;
        } else if let Some(position) = star {
            // Backtrack: let the last `*` swallow one more character.
            p = position + 1;
            resume += 1;
            n = resume;
        } else {
            return false;
        }
    }

    while p < pattern.len() && pattern[p] == '*' {
        p += 1;
    }
    p == pattern.len()
}

/// Executable paths are compared case-insensitively, as Windows does, and with
/// separators normalised so a path from one API matches one from another.
fn normalise(path: &str) -> String {
    path.replace('/', "\\").to_lowercase()
}

fn file_name(path: &str) -> String {
    path.rsplit('\\').next().unwrap_or(path).to_owned()
}

/// The file name without its extension, title-cased enough to look deliberate.
fn file_stem(path: &str) -> String {
    let name = file_name(path);
    let stem = name.strip_suffix(".exe").unwrap_or(&name);
    let mut characters = stem.chars();
    match characters.next() {
        Some(first) => first.to_uppercase().collect::<String>() + characters.as_str(),
        None => stem.to_owned(),
    }
}

/// How far the dock is pushed off the bottom of the screen while hidden.
///
/// Everything but `hover_region` is off screen; that strip is what the pointer
/// has to reach to bring it back, so a zero-height region would make an
/// auto-hiding dock unreachable.
pub fn hidden_offset(dock_height: f64, hover_region: f64) -> f64 {
    (dock_height - hover_region.max(1.0)).max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn window(executable: &str, title: &str, active: bool) -> WindowInfo {
        WindowInfo {
            id: format!("{executable}-{title}"),
            title: title.to_owned(),
            executable: executable.to_owned(),
            name: file_stem(executable),
            icon: format!("{executable}.png"),
            active,
        }
    }

    #[test]
    fn windows_of_one_program_share_an_icon() {
        let windows = [
            window(r"C:\Program Files\Firefox\firefox.exe", "Inbox", false),
            window(r"C:\Program Files\Firefox\firefox.exe", "Docs", true),
            window(r"C:\Windows\explorer.exe", "Downloads", false),
        ];

        let apps = group(&windows, &[], &[]);
        assert_eq!(apps.len(), 2);
        assert_eq!(apps[0].windows.len(), 2);
        // One window being foreground makes the whole icon active.
        assert!(apps[0].active);
        assert!(!apps[1].active);
    }

    #[test]
    fn the_same_program_from_two_spellings_is_one_icon() {
        // Different Win32 calls hand back different casing and separators; a
        // dock that showed Firefox twice because of that would look broken.
        let windows = [
            window(r"C:\Apps\Firefox.exe", "One", false),
            window(r"c:/apps/firefox.exe", "Two", false),
        ];
        let apps = group(&windows, &[], &[]);
        assert_eq!(apps.len(), 1);
        assert_eq!(apps[0].windows.len(), 2);
    }

    #[test]
    fn pinned_applications_come_first_and_stay_when_closed() {
        let pinned = [r"C:\Apps\Editor.exe".to_owned()];
        let windows = [window(r"C:\Apps\Firefox.exe", "Web", false)];

        let apps = group(&windows, &pinned, &[]);
        assert_eq!(apps.len(), 2);
        assert!(apps[0].pinned);
        assert_eq!(apps[0].name, "Editor");
        // Nothing running, so it draws without a running dot.
        assert!(apps[0].windows.is_empty());
        assert!(!apps[1].pinned);
    }

    #[test]
    fn a_pinned_application_that_is_running_is_one_icon_not_two() {
        let pinned = [r"C:\Apps\Editor.exe".to_owned()];
        let windows = [window(r"C:\Apps\Editor.exe", "main.rs", true)];

        let apps = group(&windows, &pinned, &[]);
        assert_eq!(apps.len(), 1);
        assert!(apps[0].pinned);
        assert_eq!(apps[0].windows.len(), 1);
        // And the running window supplies the icon the pinned entry lacked.
        assert!(!apps[0].icon.is_empty());
    }

    #[test]
    fn duplicate_pins_do_not_produce_duplicate_icons() {
        let pinned = [
            r"C:\Apps\Editor.exe".to_owned(),
            r"c:\apps\editor.exe".to_owned(),
        ];
        assert_eq!(group(&[], &pinned, &[]).len(), 1);
    }

    #[test]
    fn ignored_executables_never_reach_the_dock() {
        let windows = [
            window(r"C:\Apps\Firefox.exe", "Web", false),
            window(r"C:\Windows\msedgewebview2.exe", "", false),
        ];
        let ignored = ["msedgewebview2.exe".to_owned()];

        let apps = group(&windows, &[], &ignored);
        assert_eq!(apps.len(), 1);
        assert_eq!(apps[0].name, "Firefox");
    }

    #[test]
    fn ignore_patterns_are_globs_and_case_insensitive() {
        assert!(matches_glob("*host.exe", "svchost.exe"));
        assert!(matches_glob("*.tmp", "installer.tmp"));
        assert!(matches_glob("app?.exe", "app1.exe"));
        assert!(!matches_glob("app?.exe", "app12.exe"));
        assert!(!matches_glob("*host.exe", "firefox.exe"));

        // Casing comes from either side; the caller lowercases both.
        let windows = [window(r"C:\Windows\SvcHost.exe", "", false)];
        assert!(group(&windows, &[], &["*HOST.EXE".to_owned()]).is_empty());
    }

    #[test]
    fn a_bare_star_matches_everything_and_an_empty_pattern_matches_nothing() {
        assert!(matches_glob("*", "anything.exe"));
        assert!(!matches_glob("", "anything.exe"));
        assert!(matches_glob("", ""));
    }

    #[test]
    fn a_pathological_pattern_still_terminates() {
        // Naive recursive globbing goes exponential on this shape.
        assert!(!matches_glob(
            "*a*a*a*a*a*a*b",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
        ));
    }

    #[test]
    fn an_auto_hiding_dock_always_leaves_something_to_reach_for() {
        assert_eq!(hidden_offset(60.0, 3.0), 57.0);
        // A zero-height hover region would put the whole dock off screen with
        // no way to bring it back.
        assert_eq!(hidden_offset(60.0, 0.0), 59.0);
        // And a region larger than the dock means it never hides.
        assert_eq!(hidden_offset(60.0, 100.0), 0.0);
    }
}
