//! Browsing the local wallpaper folder.
//!
//! Replaces the `Qt.labs.folderlistmodel` half of end4-pC's `services/Wallpapers.qml`:
//! directory listing filtered by extension, plus the work-safety keyword check
//! that blanks a wallpaper rather than showing it.

pub mod online;

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use ts_rs::TS;

/// One entry in the wallpaper browser — a folder to descend into, or an image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Entry {
    pub path: String,
    pub name: String,
    pub is_directory: bool,
}

/// Lists one directory: folders first, then images, each alphabetically.
///
/// Unreadable entries are skipped rather than failing the whole listing — a
/// single permission-denied file should not empty the browser.
pub fn list_directory(dir: &Path, extensions: &[String]) -> std::io::Result<Vec<Entry>> {
    let mut directories = Vec::new();
    let mut files = Vec::new();

    for entry in std::fs::read_dir(dir)? {
        let Ok(entry) = entry else { continue };
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        // Hidden and system folders are noise in a picture browser.
        if name.starts_with('.') {
            continue;
        }
        let Ok(file_type) = entry.file_type() else {
            continue;
        };

        if file_type.is_dir() {
            directories.push(Entry {
                path: path.to_string_lossy().into_owned(),
                name: name.to_owned(),
                is_directory: true,
            });
        } else if has_extension(&path, extensions) {
            files.push(Entry {
                path: path.to_string_lossy().into_owned(),
                name: name.to_owned(),
                is_directory: false,
            });
        }
    }

    let by_name = |a: &Entry, b: &Entry| a.name.to_lowercase().cmp(&b.name.to_lowercase());
    directories.sort_by(by_name);
    files.sort_by(by_name);
    directories.extend(files);
    Ok(directories)
}

fn has_extension(path: &Path, extensions: &[String]) -> bool {
    let Some(found) = path.extension().and_then(|e| e.to_str()) else {
        return false;
    };
    extensions
        .iter()
        .any(|wanted| wanted.eq_ignore_ascii_case(found))
}

/// Whether a wallpaper should be blanked because its path matches a work-safety
/// keyword. Matching is case-insensitive and covers the whole path, so a
/// keyword can name a folder as well as a file.
pub fn is_work_unsafe(path: &Path, keywords: &[String]) -> bool {
    if keywords.is_empty() {
        return false;
    }
    let haystack = path.to_string_lossy().to_lowercase();
    keywords
        .iter()
        .filter(|keyword| !keyword.trim().is_empty())
        .any(|keyword| haystack.contains(&keyword.to_lowercase()))
}

/// Picks a wallpaper from `entries` other than `current`.
///
/// Returns `None` when there is nothing to switch to, so a folder holding a
/// single image does not "rotate" onto itself.
pub fn next_random(entries: &[Entry], current: Option<&str>, seed: u64) -> Option<PathBuf> {
    let candidates: Vec<&Entry> = entries
        .iter()
        .filter(|entry| !entry.is_directory)
        .filter(|entry| Some(entry.path.as_str()) != current)
        .collect();

    if candidates.is_empty() {
        return None;
    }
    // A tiny xorshift keeps this deterministic for tests without pulling in a
    // random-number dependency for one call site.
    let mut state = seed | 1;
    state ^= state << 13;
    state ^= state >> 7;
    state ^= state << 17;
    let index = (state % candidates.len() as u64) as usize;
    Some(PathBuf::from(&candidates[index].path))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn extensions() -> Vec<String> {
        ["jpg", "png", "webp"].map(str::to_owned).to_vec()
    }

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("bw-wall-{}-{name}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn listing_puts_folders_first_and_filters_by_extension() {
        let dir = scratch("listing");
        std::fs::create_dir_all(dir.join("Nature")).unwrap();
        for file in ["b.png", "a.JPG", "notes.txt", "clip.mp4"] {
            std::fs::write(dir.join(file), b"x").unwrap();
        }

        let entries = list_directory(&dir, &extensions()).unwrap();
        let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, vec!["Nature", "a.JPG", "b.png"]);
        assert!(entries[0].is_directory);
        assert!(!entries[1].is_directory);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn hidden_entries_are_skipped() {
        let dir = scratch("hidden");
        std::fs::write(dir.join(".secret.png"), b"x").unwrap();
        std::fs::write(dir.join("shown.png"), b"x").unwrap();
        let entries = list_directory(&dir, &extensions()).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "shown.png");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_missing_directory_is_an_error_not_an_empty_list() {
        let missing = std::env::temp_dir().join("bw-wall-nope-nope");
        assert!(list_directory(&missing, &extensions()).is_err());
    }

    #[test]
    fn work_safety_matches_folders_and_is_case_insensitive() {
        let keywords = vec!["Waifu".to_owned()];
        assert!(is_work_unsafe(Path::new(r"C:\Pics\waifu\a.png"), &keywords));
        assert!(is_work_unsafe(Path::new(r"C:\Pics\WAIFU-2.png"), &keywords));
        assert!(!is_work_unsafe(Path::new(r"C:\Pics\hills.png"), &keywords));
        assert!(!is_work_unsafe(Path::new(r"C:\Pics\waifu\a.png"), &[]));
    }

    #[test]
    fn blank_keywords_do_not_match_everything() {
        let keywords = vec![String::new(), "  ".to_owned()];
        assert!(!is_work_unsafe(Path::new(r"C:\Pics\hills.png"), &keywords));
    }

    #[test]
    fn rotation_never_picks_the_current_wallpaper() {
        let entries: Vec<Entry> = ["a.png", "b.png", "c.png"]
            .iter()
            .map(|name| Entry {
                path: format!(r"C:\W\{name}"),
                name: (*name).to_owned(),
                is_directory: false,
            })
            .collect();

        for seed in 0..64u64 {
            let picked = next_random(&entries, Some(r"C:\W\a.png"), seed).unwrap();
            assert_ne!(picked, PathBuf::from(r"C:\W\a.png"));
        }
    }

    #[test]
    fn rotation_gives_up_when_there_is_nowhere_to_go() {
        let only = vec![Entry {
            path: r"C:\W\a.png".into(),
            name: "a.png".into(),
            is_directory: false,
        }];
        assert!(next_random(&only, Some(r"C:\W\a.png"), 7).is_none());
        assert!(next_random(&[], None, 7).is_none());
    }

    #[test]
    fn rotation_ignores_directories() {
        let entries = vec![
            Entry {
                path: r"C:\W\sub".into(),
                name: "sub".into(),
                is_directory: true,
            },
            Entry {
                path: r"C:\W\a.png".into(),
                name: "a.png".into(),
                is_directory: false,
            },
        ];
        assert_eq!(
            next_random(&entries, None, 3),
            Some(PathBuf::from(r"C:\W\a.png"))
        );
    }
}
