//! What goes in the Run key, and whether what is there is still ours.
//!
//! The original has nothing to do here: a Hyprland session starts Quickshell
//! from `hyprland.conf`, and the window manager owns the question. On Windows
//! a shell that wants to come back after a reboot registers itself, and the
//! per-user Run key is the right place for an installation that went to
//! `%LOCALAPPDATA%` for one user rather than to `Program Files` for everyone.
//!
//! Three things about that value are easy to get wrong and all three fail
//! quietly at the worst moment — the next login, before anything of the shell
//! is on screen to explain itself. So they live here, under tests.

/// The value to write into the Run key for an executable.
///
/// Quoted, always. `C:\Program Files\beautiful-wallpaper\bw.exe` without
/// quotes is read by Windows as `C:\Program.exe` with `Files\...` as its
/// argument — the single most common way an auto-start entry fails, and it
/// fails at login where nothing of this shell exists yet to say why.
///
/// And deliberately with **no arguments**: `bw.exe` with none starts the
/// shell, and `bw.exe` with any at all is a CLI client that talks to a running
/// one. A well-meant `--startup` here would make every login print
/// "beautiful-wallpaper is not running" and exit.
pub fn command_line(executable: &str) -> String {
    format!("\"{}\"", executable.trim().trim_matches('"'))
}

/// Whether an existing Run entry points at this executable.
///
/// Compared loosely on purpose. Windows paths are case-insensitive and accept
/// either separator, and an entry may have been written by an older build that
/// did not quote it. What this is really looking for is the one case that
/// matters: an entry left behind by an installation that has since moved,
/// which produces an error dialog at every login for a file that is gone.
pub fn is_ours(entry: &str, executable: &str) -> bool {
    !executable.trim().is_empty() && tidy(entry) == tidy(executable)
}

fn tidy(path: &str) -> String {
    path.trim()
        .trim_matches('"')
        .trim()
        .replace('/', "\\")
        .to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SPACED: &str = r"C:\Program Files\beautiful-wallpaper\bw.exe";

    /// The failure this exists for: unquoted, Windows runs `C:\Program.exe`.
    #[test]
    fn the_command_line_is_quoted() {
        assert_eq!(command_line(SPACED), format!("\"{SPACED}\""));
    }

    /// `bw.exe` with any argument is a CLI client, so a login that passed one
    /// would print "not running" and exit instead of starting the shell.
    #[test]
    fn the_command_line_carries_no_arguments() {
        let written = command_line(SPACED);
        assert!(written.ends_with("bw.exe\""), "{written}");
        assert_eq!(written.matches('"').count(), 2);
    }

    #[test]
    fn quoting_something_already_quoted_does_not_double_it() {
        assert_eq!(
            command_line(&format!("\"{SPACED}\"")),
            format!("\"{SPACED}\"")
        );
        assert_eq!(
            command_line(&format!("  {SPACED}  ")),
            format!("\"{SPACED}\"")
        );
    }

    #[test]
    fn an_entry_is_ours_however_it_was_written() {
        assert!(is_ours(&command_line(SPACED), SPACED));
        // An older build that did not quote it.
        assert!(is_ours(SPACED, SPACED));
        // Windows paths are case-insensitive and take either separator.
        assert!(is_ours(
            r"c:\program files\Beautiful-Wallpaper\BW.EXE",
            SPACED
        ));
        assert!(is_ours(
            r"C:/Program Files/beautiful-wallpaper/bw.exe",
            SPACED
        ));
        assert!(is_ours(&format!("  \"{SPACED}\" "), SPACED));
    }

    /// The case worth detecting: the installation moved, and the entry now
    /// names a file that is not there. Left alone it is an error dialog at
    /// every login.
    #[test]
    fn an_entry_from_a_previous_installation_is_not_ours() {
        assert!(!is_ours(
            r"C:\Users\me\AppData\Local\beautiful-wallpaper\bw.exe",
            SPACED
        ));
        assert!(!is_ours(r"C:\Windows\notepad.exe", SPACED));
        // A different program's entry that merely starts the same way.
        assert!(!is_ours(
            r"C:\Program Files\beautiful-wallpaper\bw-helper.exe",
            SPACED
        ));
    }

    #[test]
    fn nothing_matches_an_executable_we_do_not_know() {
        assert!(!is_ours(SPACED, ""));
        assert!(!is_ours(SPACED, "   "));
        assert!(!is_ours("", ""));
    }
}
