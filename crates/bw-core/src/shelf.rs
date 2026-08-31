//! The drop shelf: somewhere to put files down for a moment.
//!
//! The whole point of a shelf is moving a file between two places that are
//! never on screen at the same time — out of a mail attachment, on to the
//! desktop, into a chat window three minutes later. So it holds **paths, not
//! copies**. Copying would duplicate gigabytes for a gesture that is meant to
//! be free, and it would leave the user editing a copy while believing they
//! were editing the original.
//!
//! The cost of holding paths is that the thing behind one can go away, and
//! that is treated as something to say rather than something to hide: an entry
//! whose file has moved stays on the shelf, marked, instead of vanishing and
//! leaving the user wondering whether they ever put it there.
//!
//! Everything derived from the path — the name, what kind of thing it is, how
//! big — is worked out when the shelf is read rather than stored. A cached
//! name is a name that can disagree with the disk, and there is nothing to
//! gain from the disagreement.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// One thing on the shelf, as the surface draws it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ShelfItem {
    /// Stable across reorders and reloads, unlike a position in the list.
    pub id: u32,
    pub path: String,
    /// The last segment of the path — what the user calls the file.
    pub name: String,
    pub kind: ShelfKind,
    /// Bytes. `None` for a folder, and for anything that cannot be read.
    ///
    /// Typed as a plain number on the TypeScript side rather than the `bigint`
    /// a `u64` would generate: serde writes it as a JSON number, so a `bigint`
    /// would be a lie about what actually arrives. A file large enough to lose
    /// precision in a double is nine petabytes.
    #[ts(type = "number | null")]
    pub size: Option<u64>,
    /// The path no longer leads anywhere. Kept, and said, rather than dropped.
    pub missing: bool,
}

/// Enough of a distinction to pick a glyph by.
///
/// Deliberately coarse: this exists to make a list of twenty files scannable,
/// not to identify formats. Anything unrecognised is a file, which is true.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ShelfKind {
    Folder,
    Image,
    Video,
    Audio,
    Document,
    Archive,
    Code,
    Other,
}

impl ShelfKind {
    /// What kind of thing this path names.
    pub fn of(path: &str, is_directory: bool) -> Self {
        if is_directory {
            return Self::Folder;
        }
        match extension(path).as_str() {
            "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" | "svg" | "avif" | "ico" | "tiff"
            | "heic" => Self::Image,
            "mp4" | "mkv" | "webm" | "mov" | "avi" | "wmv" | "m4v" | "flv" => Self::Video,
            "mp3" | "flac" | "wav" | "ogg" | "opus" | "m4a" | "aac" | "wma" => Self::Audio,
            "pdf" | "doc" | "docx" | "odt" | "rtf" | "txt" | "md" | "xls" | "xlsx" | "csv"
            | "ppt" | "pptx" => Self::Document,
            "zip" | "7z" | "rar" | "tar" | "gz" | "xz" | "zst" | "bz2" | "cab" | "iso" => {
                Self::Archive
            }
            "rs" | "ts" | "tsx" | "js" | "jsx" | "py" | "go" | "c" | "h" | "cpp" | "cs"
            | "java" | "rb" | "sh" | "ps1" | "json" | "toml" | "yaml" | "yml" | "html" | "css"
            | "qml" => Self::Code,
            _ => Self::Other,
        }
    }

    /// The Material Symbols name, mirrored by the surface.
    pub fn symbol(self) -> &'static str {
        match self {
            Self::Folder => "folder",
            Self::Image => "image",
            Self::Video => "movie",
            Self::Audio => "music_note",
            Self::Document => "description",
            Self::Archive => "folder_zip",
            Self::Code => "code",
            Self::Other => "draft",
        }
    }
}

/// What a drop did.
///
/// Three numbers rather than a count, because the three mean different things
/// to the person who just let go of twenty files and sees eight on the shelf.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DropOutcome {
    pub added: u32,
    /// Already here, and moved back to the top rather than duplicated.
    pub moved: u32,
    /// Turned away because the shelf is full. **Not** silently discarded:
    /// nothing already on the shelf is thrown out to make room, and the count
    /// is reported so the difference is visible.
    pub refused: u32,
}

impl DropOutcome {
    pub fn nothing_happened(self) -> bool {
        self.added == 0 && self.moved == 0 && self.refused == 0
    }
}

/// The last segment of a path, whichever separator it uses.
///
/// Not `Path::file_name`: these are Windows paths, and this crate's tests run
/// on Linux, where `Path` does not treat `\` as a separator at all. Using the
/// standard function would make every name the entire path — in the tests, and
/// on any host that is not Windows.
pub fn file_name_of(path: &str) -> String {
    let trimmed = path.trim_end_matches(['/', '\\']);
    match trimmed.rsplit(['/', '\\']).next() {
        // A bare drive letter, a root, or nothing at all: there is no last
        // segment to show, so show what there is.
        Some(name) if !name.is_empty() => name.to_owned(),
        _ => path.to_owned(),
    }
}

fn extension(path: &str) -> String {
    let name = file_name_of(path);
    match name.rsplit_once('.') {
        // A leading dot is the whole name of a dotfile, not an extension.
        Some((stem, extension)) if !stem.is_empty() => extension.to_ascii_lowercase(),
        _ => String::new(),
    }
}

/// Whether two paths name the same thing, as Windows would judge it.
///
/// Case-insensitive, and `/` and `\` are the same separator — both because
/// Windows says so, and because a path typed by hand into the config will not
/// match one Explorer handed over otherwise.
pub fn same_path(left: &str, right: &str) -> bool {
    let normalise = |value: &str| {
        value
            .trim_end_matches(['/', '\\'])
            .chars()
            .map(|character| match character {
                '/' => '\\',
                other => other.to_ascii_lowercase(),
            })
            .collect::<String>()
    };
    normalise(left) == normalise(right)
}

/// What is written to disk: the paths, and nothing derived from them.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Entry {
    id: u32,
    path: String,
}

pub struct Store {
    inner: Mutex<Inner>,
    path: PathBuf,
}

struct Inner {
    entries: Vec<Entry>,
    next_id: u32,
}

impl Store {
    /// Loads the shelf from `path`, starting empty if it cannot be read.
    pub fn load(path: PathBuf) -> Self {
        let entries: Vec<Entry> = std::fs::read_to_string(&path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default();

        let next_id = entries
            .iter()
            .map(|entry| entry.id)
            .max()
            .unwrap_or(0)
            .saturating_add(1);

        Self {
            inner: Mutex::new(Inner { entries, next_id }),
            path,
        }
    }

    /// Recovers from a poisoned lock rather than panicking, as the other
    /// stores do: a shelf is not worth taking the shell down for.
    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// The shelf as the surface should draw it, newest first.
    ///
    /// The disk is consulted here rather than at drop time: a file that was
    /// there when it was put on the shelf can be gone by the time the shelf is
    /// opened, and that is the state worth showing.
    pub fn list(&self) -> Vec<ShelfItem> {
        self.lock()
            .entries
            .iter()
            .map(|entry| describe(entry.id, &entry.path))
            .collect()
    }

    pub fn paths(&self) -> Vec<String> {
        self.lock()
            .entries
            .iter()
            .map(|entry| entry.path.clone())
            .collect()
    }

    /// Puts paths on the shelf, newest first, and says what became of them.
    ///
    /// A path already here is moved back to the top instead of appearing
    /// twice; dropping the same folder again is how someone says "this one",
    /// not how they ask for two of it.
    ///
    /// When there is not room for everything, the **new** items are the ones
    /// turned away. Making room by discarding what is already on the shelf
    /// would throw away something the user put there deliberately in order to
    /// keep something they may have selected by accident.
    pub fn add(&self, paths: &[String], max_items: usize) -> DropOutcome {
        let mut outcome = DropOutcome::default();
        let mut inner = self.lock();

        // Kept in the order they arrived: dropping three files should list
        // them the way they were selected, not backwards.
        let mut front: Vec<Entry> = Vec::new();

        for path in paths {
            let path = path.trim();
            if path.is_empty() {
                continue;
            }
            // The same path twice in one drop is one file, not two.
            if front.iter().any(|entry| same_path(&entry.path, path)) {
                continue;
            }

            if let Some(position) = inner
                .entries
                .iter()
                .position(|entry| same_path(&entry.path, path))
            {
                // The id stays — the surface addresses entries by it, and this
                // is the same entry moving, not a new one. The spelling is
                // taken from the drop: two spellings of one path are equally
                // valid on Windows, and the one that just arrived came
                // straight from the program the file was dragged out of.
                let mut existing = inner.entries.remove(position);
                existing.path = path.to_owned();
                front.push(existing);
                outcome.moved += 1;
                continue;
            }

            if front.len() + inner.entries.len() >= max_items {
                outcome.refused += 1;
                continue;
            }

            front.push(Entry {
                id: inner.next_id,
                path: path.to_owned(),
            });
            inner.next_id = inner.next_id.saturating_add(1);
            outcome.added += 1;
        }

        if outcome.nothing_happened() {
            return outcome;
        }

        front.append(&mut inner.entries);
        inner.entries = front;
        self.persist_from(inner);
        outcome
    }

    pub fn remove(&self, id: u32) -> bool {
        let mut inner = self.lock();
        let before = inner.entries.len();
        inner.entries.retain(|entry| entry.id != id);
        if inner.entries.len() == before {
            return false;
        }
        self.persist_from(inner);
        true
    }

    /// Empties the shelf. Returns how many were taken off it.
    pub fn clear(&self) -> usize {
        let mut inner = self.lock();
        let removed = inner.entries.len();
        if removed > 0 {
            inner.entries.clear();
            self.persist_from(inner);
        }
        removed
    }

    /// Drops every entry whose file is no longer there.
    ///
    /// Only ever on the user's say-so — doing it automatically is how a shelf
    /// silently empties itself while a network drive is disconnected.
    pub fn clear_missing(&self) -> usize {
        let mut inner = self.lock();
        let before = inner.entries.len();
        inner
            .entries
            .retain(|entry| !describe(entry.id, &entry.path).missing);
        let removed = before - inner.entries.len();
        if removed > 0 {
            self.persist_from(inner);
        }
        removed
    }

    /// The path behind one entry, for the actions that need it.
    pub fn path_of(&self, id: u32) -> Option<String> {
        self.lock()
            .entries
            .iter()
            .find(|entry| entry.id == id)
            .map(|entry| entry.path.clone())
    }

    /// Writes the shelf out, releasing the lock first so a slow disk cannot
    /// block a reader.
    fn persist_from(&self, inner: std::sync::MutexGuard<'_, Inner>) {
        let snapshot: Vec<Entry> = inner.entries.clone();
        drop(inner);

        if let Some(parent) = self.path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        // Failing to persist is not worth interrupting the user over: the
        // shelf is already correct in memory and already on screen.
        if let Ok(text) = serde_json::to_string_pretty(&snapshot) {
            let _ = std::fs::write(&self.path, text);
        }
    }
}

/// Everything about a path that the shelf shows, worked out from the disk.
fn describe(id: u32, path: &str) -> ShelfItem {
    let on_disk = Path::new(path);
    let metadata = on_disk.metadata().ok();
    let is_directory = metadata.as_ref().is_some_and(std::fs::Metadata::is_dir);

    ShelfItem {
        id,
        name: file_name_of(path),
        kind: ShelfKind::of(path, is_directory),
        size: metadata
            .as_ref()
            .filter(|found| found.is_file())
            .map(std::fs::Metadata::len),
        missing: metadata.is_none(),
        path: path.to_owned(),
    }
}

/// Where the shelf lives, alongside the other state the shell keeps.
pub fn shelf_path() -> PathBuf {
    crate::paths::state_dir().join("shelf.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("bw-shelf-{name}"));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("a temp directory");
        path
    }

    fn store_in(directory: &Path) -> Store {
        Store::load(directory.join("shelf.json"))
    }

    fn owned(paths: &[&str]) -> Vec<String> {
        paths.iter().map(|path| (*path).to_owned()).collect()
    }

    /// The trap this module exists around: these are Windows paths, and this
    /// test runs on Linux, where `Path::file_name` sees no separator at all.
    #[test]
    fn a_windows_path_still_has_a_last_segment_off_windows() {
        assert_eq!(file_name_of(r"C:\Users\you\report.pdf"), "report.pdf");
        assert_eq!(file_name_of(r"C:\Users\you\Pictures"), "Pictures");
        // A trailing separator is not a nameless entry.
        assert_eq!(file_name_of(r"C:\Users\you\Pictures\"), "Pictures");
        assert_eq!(file_name_of("/home/you/report.pdf"), "report.pdf");
    }

    /// A drive or a root has no file name; showing the drive is better than
    /// showing nothing, which is what the standard call would give.
    #[test]
    fn a_root_shows_what_there_is_rather_than_nothing() {
        assert_eq!(file_name_of(r"C:\"), "C:");
        assert_eq!(file_name_of("/"), "/");
        assert_eq!(file_name_of(""), "");
        assert_eq!(file_name_of(r"\\server\share"), "share");
    }

    #[test]
    fn kinds_come_from_the_extension_and_ignore_its_case() {
        assert_eq!(ShelfKind::of(r"C:\a\photo.JPG", false), ShelfKind::Image);
        assert_eq!(ShelfKind::of(r"C:\a\notes.md", false), ShelfKind::Document);
        assert_eq!(ShelfKind::of(r"C:\a\main.rs", false), ShelfKind::Code);
        assert_eq!(ShelfKind::of(r"C:\a\thing", false), ShelfKind::Other);
        // A directory is a directory whatever its name looks like.
        assert_eq!(ShelfKind::of(r"C:\a\album.mp3", true), ShelfKind::Folder);
    }

    /// `.gitignore` is a name, not an extension, and is certainly not a
    /// "gitignore file type".
    #[test]
    fn a_dotfile_has_no_extension() {
        assert_eq!(ShelfKind::of(r"C:\a\.gitignore", false), ShelfKind::Other);
    }

    #[test]
    fn windows_paths_compare_without_regard_to_case_or_separator() {
        assert!(same_path(r"C:\Users\You\a.txt", r"c:/users/you/A.TXT"));
        assert!(same_path(r"C:\Users\You\", r"C:\Users\You"));
        assert!(!same_path(r"C:\Users\You\a.txt", r"C:\Users\You\b.txt"));
    }

    #[test]
    fn a_drop_lands_newest_first_and_keeps_its_own_order() {
        let directory = temp_dir("order");
        let shelf = store_in(&directory);

        shelf.add(&owned([r"C:\a\one.txt"].as_slice()), 100);
        shelf.add(&owned([r"C:\a\two.txt", r"C:\a\three.txt"].as_slice()), 100);

        let names: Vec<String> = shelf.list().into_iter().map(|item| item.name).collect();
        assert_eq!(names, ["two.txt", "three.txt", "one.txt"]);
    }

    #[test]
    fn dropping_something_already_here_moves_it_up_rather_than_duplicating_it() {
        let directory = temp_dir("dedupe");
        let shelf = store_in(&directory);

        shelf.add(&owned([r"C:\a\one.txt", r"C:\a\two.txt"].as_slice()), 100);
        // Same file, spelled the way another program might hand it over.
        let outcome = shelf.add(&owned(["c:/a/ONE.TXT"].as_slice()), 100);

        assert_eq!(
            outcome,
            DropOutcome {
                added: 0,
                moved: 1,
                refused: 0
            }
        );
        let items = shelf.list();
        let names: Vec<&str> = items.iter().map(|item| item.name.as_str()).collect();
        assert_eq!(names, ["ONE.TXT", "two.txt"], "the newest spelling wins");
        assert_eq!(items[0].id, 1, "but it is the same entry, with its id");
    }

    #[test]
    fn the_same_path_twice_in_one_drop_is_one_file() {
        let directory = temp_dir("twice");
        let shelf = store_in(&directory);

        let outcome = shelf.add(&owned([r"C:\a\one.txt", r"C:\a\one.txt"].as_slice()), 100);
        assert_eq!(outcome.added, 1);
        assert_eq!(shelf.list().len(), 1);
    }

    /// A full shelf turns the new ones away. It does not quietly throw out
    /// what the user put there earlier to make room for what they may have
    /// selected by accident.
    #[test]
    fn a_full_shelf_refuses_rather_than_evicting() {
        let directory = temp_dir("full");
        let shelf = store_in(&directory);

        shelf.add(&owned([r"C:\a\kept.txt", r"C:\a\also.txt"].as_slice()), 2);
        let outcome = shelf.add(&owned([r"C:\a\late.txt"].as_slice()), 2);

        assert_eq!(
            outcome,
            DropOutcome {
                added: 0,
                moved: 0,
                refused: 1
            }
        );
        let names: Vec<String> = shelf.list().into_iter().map(|item| item.name).collect();
        assert_eq!(names, ["kept.txt", "also.txt"]);
    }

    /// Part of a large drop still lands, and the rest is counted rather than
    /// disappearing: eight of twenty on the shelf has to be explainable.
    #[test]
    fn a_partial_drop_reports_both_halves() {
        let directory = temp_dir("partial");
        let shelf = store_in(&directory);

        let outcome = shelf.add(
            &owned([r"C:\a\1", r"C:\a\2", r"C:\a\3", r"C:\a\4"].as_slice()),
            2,
        );
        assert_eq!(
            outcome,
            DropOutcome {
                added: 2,
                moved: 0,
                refused: 2
            }
        );
        assert_eq!(shelf.list().len(), 2);
    }

    /// Something already on a full shelf can still be moved to the top: it
    /// takes no room it was not already taking.
    #[test]
    fn a_full_shelf_still_moves_what_it_already_holds() {
        let directory = temp_dir("full-move");
        let shelf = store_in(&directory);

        shelf.add(&owned([r"C:\a\one", r"C:\a\two"].as_slice()), 2);
        let outcome = shelf.add(&owned([r"C:\a\two"].as_slice()), 2);

        assert_eq!(
            outcome,
            DropOutcome {
                added: 0,
                moved: 1,
                refused: 0
            }
        );
        let names: Vec<String> = shelf.list().into_iter().map(|item| item.name).collect();
        assert_eq!(names, ["two", "one"]);
    }

    #[test]
    fn a_file_that_has_gone_is_marked_rather_than_dropped() {
        let directory = temp_dir("missing");
        let here = directory.join("here.txt");
        std::fs::write(&here, b"hello").expect("a file");

        let shelf = store_in(&directory);
        shelf.add(
            &[
                here.to_string_lossy().into_owned(),
                directory.join("gone.txt").to_string_lossy().into_owned(),
            ],
            100,
        );

        let items = shelf.list();
        assert_eq!(items.len(), 2, "nothing is removed behind the user's back");
        assert!(!items[0].missing);
        assert_eq!(items[0].size, Some(5));
        assert!(items[1].missing);
        assert_eq!(items[1].size, None);

        assert_eq!(shelf.clear_missing(), 1);
        assert_eq!(shelf.list().len(), 1);
    }

    #[test]
    fn a_folder_has_no_size() {
        let directory = temp_dir("folder");
        let shelf = store_in(&directory);
        shelf.add(&[directory.to_string_lossy().into_owned()], 100);

        let item = shelf.list().pop().expect("one entry");
        assert_eq!(item.kind, ShelfKind::Folder);
        assert_eq!(item.size, None);
        assert!(!item.missing);
    }

    #[test]
    fn the_shelf_survives_a_restart() {
        let directory = temp_dir("persist");
        let first = store_in(&directory);
        first.add(&owned([r"C:\a\one.txt", r"C:\a\two.txt"].as_slice()), 100);
        let ids: Vec<u32> = first.list().into_iter().map(|item| item.id).collect();
        drop(first);

        let second = store_in(&directory);
        let reloaded: Vec<u32> = second.list().into_iter().map(|item| item.id).collect();
        assert_eq!(reloaded, ids, "ids are what the surface addresses by");

        // A new drop must not reuse an id that came back from disk.
        second.add(&owned([r"C:\a\three.txt"].as_slice()), 100);
        let mut all: Vec<u32> = second.list().into_iter().map(|item| item.id).collect();
        let before = all.len();
        all.sort_unstable();
        all.dedup();
        assert_eq!(all.len(), before, "duplicate id after a reload");
    }

    #[test]
    fn removing_and_clearing_report_what_they_did() {
        let directory = temp_dir("remove");
        let shelf = store_in(&directory);
        shelf.add(&owned([r"C:\a\one", r"C:\a\two"].as_slice()), 100);

        let id = shelf.list()[0].id;
        assert!(shelf.remove(id));
        assert!(!shelf.remove(id), "removing twice is not a second removal");
        assert_eq!(shelf.clear(), 1);
        assert_eq!(shelf.clear(), 0);
    }

    #[test]
    fn blank_paths_are_not_entries() {
        let directory = temp_dir("blank");
        let shelf = store_in(&directory);
        let outcome = shelf.add(&owned(["", "   "].as_slice()), 100);
        assert!(outcome.nothing_happened());
        assert!(shelf.list().is_empty());
    }
}
