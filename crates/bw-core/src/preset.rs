//! Named snapshots of the config, and what applying one does.
//!
//! end4-pC keeps these as bare `config.json` copies in `presets/`, saved and
//! applied by a Bash script: `jq -s '.[0] * .[1]'` deep-merges the preset over
//! the live config, and a `_presetMeta` key smuggled into the document carries
//! the description. Three things about that do not survive the crossing.
//!
//! **The metadata cannot live inside the config.** This schema is
//! `deny_unknown_fields`, so a `_presetMeta` key would make the file
//! unreadable as a config. The preset is a wrapper around the config instead,
//! which also means nothing has to remember to strip a key on the way back
//! out — forget that once upstream and the junk key lands in `config.json`.
//!
//! **A whole-file merge can write keys this build has never heard of.** A
//! preset saved by a newer version carries its new keys, and merging them in
//! produces a `config.json` that this build then refuses to load — a shell
//! that will not start, from pressing Apply. So applying is not a merge: it is
//! a list of paths, every one of which [`compare`] has already found on both
//! sides. What the preset carries and this build has no setting for is
//! reported, not written.
//!
//! **The name is a file name.** Upstream replaces whitespace with `_` and
//! stops there, which is enough for a Bash argument and not nearly enough for
//! Windows: `..` walks out of the folder, `NUL` is a device, and a trailing
//! dot is silently dropped so `Dark.` and `Dark` are the same file. Names are
//! checked here, once, under tests.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use ts_rs::TS;

use crate::config;

/// A preset as it sits on disk.
///
/// The file name is the preset's name — renaming `Dark.json` in Explorer
/// renames the preset, which is what anyone moving files around expects — so
/// nothing in here repeats it and the two cannot disagree.
///
/// Deliberately not `deny_unknown_fields`: a preset written by a newer build
/// may carry metadata this one does not know, and that is no reason to refuse
/// to read the config inside it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Preset {
    #[serde(default)]
    pub description: String,
    /// RFC 3339, local time. Only ever displayed.
    #[serde(default)]
    pub created: String,
    /// The whole config as it was when the preset was saved.
    pub config: Value,
}

/// One preset in the list.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PresetSummary {
    pub name: String,
    pub description: String,
    pub created: String,
    /// The wallpaper this preset would set, for the card's thumbnail. Empty
    /// when the preset has none.
    pub wallpaper: String,
    /// Why this preset cannot be applied, when it cannot.
    ///
    /// A file that will not parse is listed saying so rather than dropped: a
    /// preset that silently disappears looks like the shell lost it, and the
    /// user has no way to find out it is still there on disk.
    pub problem: Option<String>,
}

/// One setting a preset would change.
///
/// Carries display strings rather than values: this is a list to read before
/// pressing Apply, and the label and section come from the settings schema the
/// frontend already has, so the confirm list words a setting exactly as its
/// settings row does.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Change {
    /// The dotted path, which is also what identifies it when applying.
    pub path: String,
    pub from: String,
    pub to: String,
}

/// What applying a preset would do.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Comparison {
    /// Every setting that differs, in the schema's own order.
    pub changes: Vec<Change>,
    /// Paths the preset carries that this build has no setting for — a preset
    /// from a newer version, or from one where a key has since been renamed.
    ///
    /// Listed rather than dropped quietly, because "it applied but that part
    /// did nothing" is indistinguishable from a bug.
    pub unknown: Vec<String>,
}

/// The longest a preset name may be.
///
/// Well inside `MAX_PATH` even under a deep profile directory, and long enough
/// that nobody sensible hits it.
const MAX_NAME: usize = 64;

/// Names Windows reserves for devices, at any casing and with any extension:
/// `NUL.json` is still `NUL`.
const RESERVED: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum NameError {
    #[error("a preset needs a name")]
    Empty,
    #[error("a preset name has to be {MAX_NAME} characters or fewer")]
    TooLong,
    #[error("a preset name is a file name, so it cannot contain `\\` or `/`")]
    Separator,
    #[error("a preset name cannot contain `{0}`")]
    Character(char),
    #[error("`{0}` is a name Windows keeps for a device, so no file can have it")]
    Reserved(String),
    #[error("Windows drops a trailing dot, so `{0}` would become `{1}`")]
    TrailingDot(String, String),
}

#[derive(Debug, thiserror::Error)]
pub enum PresetError {
    #[error(transparent)]
    Name(#[from] NameError),
    #[error("there is already a preset called `{0}`")]
    Exists(String),
    #[error("there is no preset called `{0}`")]
    Missing(String),
    #[error("could not read {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("{path} is not a preset: {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error(transparent)]
    Config(#[from] config::ConfigError),
}

/// Checks a name and returns it trimmed.
///
/// Whitespace inside the name is kept. Upstream replaces it with `_` so the
/// name survives an unquoted shell argument; nothing here shells out, and
/// silently rewriting what somebody typed is a worse surprise than a space in
/// a file name — which Windows has been fine with for thirty years.
pub fn check_name(input: &str) -> Result<String, NameError> {
    let name = input.trim();

    if name.is_empty() {
        return Err(NameError::Empty);
    }
    if name.chars().count() > MAX_NAME {
        return Err(NameError::TooLong);
    }

    for character in name.chars() {
        match character {
            '\\' | '/' => return Err(NameError::Separator),
            // `:` would name an alternate data stream, the rest are simply
            // refused by the file system.
            '<' | '>' | ':' | '"' | '|' | '?' | '*' => return Err(NameError::Character(character)),
            control if (control as u32) < 0x20 => return Err(NameError::Character(control)),
            _ => {}
        }
    }

    // A trailing dot is not an error the file system reports — it is dropped,
    // so `Dark.` and `Dark` become one file and saving under the second name
    // would silently overwrite the first.
    if name.ends_with('.') {
        return Err(NameError::TrailingDot(
            name.to_owned(),
            name.trim_end_matches('.').to_owned(),
        ));
    }

    let stem = name.split('.').next().unwrap_or(name);
    if RESERVED
        .iter()
        .any(|found| found.eq_ignore_ascii_case(stem))
    {
        return Err(NameError::Reserved(stem.to_owned()));
    }

    Ok(name.to_owned())
}

/// Where a preset with this name lives.
fn file_of(dir: &Path, name: &str) -> PathBuf {
    dir.join(format!("{name}.json"))
}

/// Finds the file for a name, comparing case-insensitively.
///
/// Windows file names are case-insensitive, so `Dark` and `dark` are one
/// preset. Doing the comparison here rather than leaving it to the file system
/// means the tests — which run on Linux — exercise the same rule the shell
/// does.
fn resolve(dir: &Path, name: &str) -> Option<PathBuf> {
    for entry in std::fs::read_dir(dir).ok()?.flatten() {
        let path = entry.path();
        if name_of(&path).is_some_and(|found| found.eq_ignore_ascii_case(name)) {
            return Some(path);
        }
    }
    None
}

/// The preset name a file holds, or nothing if it is not a preset file.
fn name_of(path: &Path) -> Option<String> {
    if !path.extension().is_some_and(|ext| ext == "json") {
        return None;
    }
    Some(path.file_stem()?.to_string_lossy().into_owned())
}

/// Every preset in the folder, sorted by name.
///
/// Never fails: a folder that does not exist yet is simply empty, which is the
/// state every new installation is in.
pub fn list(dir: &Path) -> Vec<PresetSummary> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };

    let mut found: Vec<PresetSummary> = entries
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            let name = name_of(&path)?;
            Some(match read(&path) {
                Ok(preset) => PresetSummary {
                    name,
                    description: preset.description,
                    created: preset.created,
                    wallpaper: config::get_path(&preset.config, "background.wallpaperPath")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_owned(),
                    problem: None,
                },
                Err(error) => PresetSummary {
                    name,
                    description: String::new(),
                    created: String::new(),
                    wallpaper: String::new(),
                    problem: Some(error.to_string()),
                },
            })
        })
        .collect();

    found.sort_by(|left, right| {
        left.name
            .to_lowercase()
            .cmp(&right.name.to_lowercase())
            .then_with(|| left.name.cmp(&right.name))
    });
    found
}

fn read(path: &Path) -> Result<Preset, PresetError> {
    let text = std::fs::read_to_string(path).map_err(|source| PresetError::Io {
        path: path.to_owned(),
        source,
    })?;
    serde_json::from_str(&text).map_err(|source| PresetError::Parse {
        path: path.to_owned(),
        source,
    })
}

/// Reads one preset by name.
pub fn load(dir: &Path, name: &str) -> Result<Preset, PresetError> {
    let path = resolve(dir, name).ok_or_else(|| PresetError::Missing(name.to_owned()))?;
    read(&path)
}

/// Writes the config out under a name.
///
/// Refuses an existing name unless told to overwrite, because a preset is
/// something somebody built deliberately and there is no undo for a file that
/// has been replaced. The caller is expected to ask first and come back.
pub fn save(
    dir: &Path,
    name: &str,
    description: &str,
    config: &Value,
    overwrite: bool,
) -> Result<PathBuf, PresetError> {
    let name = check_name(name)?;

    let existing = resolve(dir, &name);
    if existing.is_some() && !overwrite {
        return Err(PresetError::Exists(name));
    }

    let preset = Preset {
        description: description.trim().to_owned(),
        created: chrono::Local::now().to_rfc3339(),
        config: config.clone(),
    };

    std::fs::create_dir_all(dir).map_err(|source| PresetError::Io {
        path: dir.to_owned(),
        source,
    })?;

    // Overwriting reuses the file that is already there, so renaming `dark` to
    // `Dark` in Explorer and saving again replaces it rather than leaving two
    // files that Windows would not let both exist.
    let path = existing.unwrap_or_else(|| file_of(dir, &name));
    let mut text = serde_json::to_string_pretty(&preset).expect("a preset is serialisable");
    text.push('\n');
    std::fs::write(&path, text).map_err(|source| PresetError::Io {
        path: path.clone(),
        source,
    })?;

    Ok(path)
}

/// Deletes a preset.
pub fn remove(dir: &Path, name: &str) -> Result<(), PresetError> {
    let path = resolve(dir, name).ok_or_else(|| PresetError::Missing(name.to_owned()))?;
    std::fs::remove_file(&path).map_err(|source| PresetError::Io { path, source })
}

/// What applying `preset` over `current` would change.
///
/// Walks the live config, which is the authority on what settings exist. A key
/// the preset leaves out keeps its current value — never deleted, because a
/// preset saved by an older build must not take away settings it never knew
/// about. A key the preset has and the config does not is [`Comparison::unknown`].
pub fn compare(current: &Value, preset: &Value) -> Comparison {
    let mine = leaves(current);
    let theirs = leaves(preset);

    let changes = mine
        .iter()
        .filter_map(|(path, value)| {
            let incoming = theirs
                .iter()
                .find(|(other, _)| other == path)
                .map(|(_, value)| value)?;
            if incoming == value {
                return None;
            }
            Some(Change {
                path: path.clone(),
                from: shown(value),
                to: shown(incoming),
            })
        })
        .collect();

    let unknown = theirs
        .iter()
        .filter(|(path, _)| !mine.iter().any(|(mine, _)| mine == path))
        .map(|(path, _)| path.clone())
        .collect();

    Comparison { changes, unknown }
}

/// Copies the named paths out of a preset onto a config.
///
/// Returns how many were written. Every path is expected to be one [`compare`]
/// offered — anything else is refused by [`config::set_path`] rather than
/// growing the config a key nothing reads.
pub fn apply(
    current: &mut Value,
    preset: &Value,
    paths: &[String],
) -> Result<usize, config::ConfigError> {
    let mut written = 0;
    for path in paths {
        let incoming = config::get_path(preset, path)
            .ok_or_else(|| config::ConfigError::UnknownPath(path.clone()))?
            .clone();
        config::set_path(current, path, incoming)?;
        written += 1;
    }
    Ok(written)
}

/// Every leaf of a config tree, as a dotted path.
///
/// An array is a leaf: merging `bar.left` element by element would produce a
/// list of widgets nobody chose.
fn leaves(root: &Value) -> Vec<(String, &Value)> {
    let mut found = Vec::new();
    walk(root, String::new(), &mut found);
    found
}

fn walk<'a>(node: &'a Value, path: String, into: &mut Vec<(String, &'a Value)>) {
    match node {
        Value::Object(children) => {
            for (key, child) in children {
                let next = if path.is_empty() {
                    key.clone()
                } else {
                    format!("{path}.{key}")
                };
                walk(child, next, into);
            }
        }
        // The root itself is never a leaf: a preset whose `config` is a bare
        // number has nothing to offer, and `""` is not a settable path.
        _ if path.is_empty() => {}
        _ => into.push((path, node)),
    }
}

/// A value as the confirm list shows it.
///
/// Strings are shown bare — quoting every path and font name would make the
/// list unreadable — and everything else is its JSON, which is what the
/// settings screen shows too.
fn shown(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Config;

    fn defaults() -> Value {
        serde_json::to_value(Config::default()).expect("the config is serialisable")
    }

    /// A directory of its own per test, since these touch the disk.
    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("bw-preset-{}-{tag}", std::process::id()));
        std::fs::remove_dir_all(&dir).ok();
        std::fs::create_dir_all(&dir).expect("could not make a scratch directory");
        dir
    }

    #[test]
    fn an_ordinary_name_is_kept_as_typed() {
        assert_eq!(check_name("Midnight blue").unwrap(), "Midnight blue");
        assert_eq!(check_name("  Trimmed  ").unwrap(), "Trimmed");
        // Not ASCII, and no reason it should not be a file name.
        assert_eq!(check_name("夜").unwrap(), "夜");
    }

    /// The name becomes a file name, so this is the difference between saving
    /// a preset and writing a file wherever the string says.
    #[test]
    fn a_name_cannot_walk_out_of_the_folder() {
        assert_eq!(check_name(r"..\..\config"), Err(NameError::Separator));
        assert_eq!(check_name("../evil"), Err(NameError::Separator));
        assert_eq!(
            check_name(".."),
            Err(NameError::TrailingDot("..".into(), String::new()))
        );
    }

    #[test]
    fn a_name_cannot_be_a_device() {
        assert_eq!(check_name("NUL"), Err(NameError::Reserved("NUL".into())));
        assert_eq!(check_name("con"), Err(NameError::Reserved("con".into())));
        // Windows looks at the stem, so an extension does not rescue it.
        assert_eq!(
            check_name("COM1.dark"),
            Err(NameError::Reserved("COM1".into()))
        );
        // A device name with more after it is an ordinary name.
        assert!(check_name("NULL").is_ok());
        assert!(check_name("Console").is_ok());
    }

    /// Windows drops the dot rather than refusing the name, so without this
    /// saving `Dark.` would quietly overwrite `Dark`.
    #[test]
    fn a_trailing_dot_is_refused_rather_than_dropped() {
        assert_eq!(
            check_name("Dark."),
            Err(NameError::TrailingDot("Dark.".into(), "Dark".into()))
        );
        assert!(check_name("v1.2").is_ok(), "a dot inside is fine");
    }

    #[test]
    fn the_characters_a_file_name_cannot_hold_are_refused() {
        for bad in ['<', '>', ':', '"', '|', '?', '*'] {
            assert_eq!(
                check_name(&format!("a{bad}b")),
                Err(NameError::Character(bad)),
                "`{bad}` should be refused"
            );
        }
        assert_eq!(check_name("a\nb"), Err(NameError::Character('\n')));
        assert_eq!(check_name("   "), Err(NameError::Empty));
        assert_eq!(
            check_name(&"x".repeat(MAX_NAME + 1)),
            Err(NameError::TooLong)
        );
        assert!(check_name(&"x".repeat(MAX_NAME)).is_ok());
    }

    #[test]
    fn a_saved_preset_reads_back() {
        let dir = scratch("round-trip");
        let mut config = Config::default();
        config.bar.height = 44;
        let value = serde_json::to_value(&config).unwrap();

        save(&dir, "Slim bar", "a thinner bar", &value, false).unwrap();
        let preset = load(&dir, "Slim bar").unwrap();

        assert_eq!(preset.description, "a thinner bar");
        assert!(!preset.created.is_empty(), "it records when it was saved");
        assert_eq!(preset.config, value);

        let listed = list(&dir);
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].name, "Slim bar");
        assert_eq!(listed[0].description, "a thinner bar");
        assert_eq!(listed[0].problem, None);

        std::fs::remove_dir_all(&dir).ok();
    }

    /// Two files differing only in case cannot both exist on Windows, so the
    /// second save has to find the first rather than making one.
    #[test]
    fn a_name_matches_whatever_its_case() {
        let dir = scratch("case");
        save(&dir, "Dark", "", &defaults(), false).unwrap();

        assert!(matches!(
            save(&dir, "dark", "", &defaults(), false),
            Err(PresetError::Exists(_))
        ));
        save(&dir, "DARK", "louder", &defaults(), true).unwrap();

        let listed = list(&dir);
        assert_eq!(listed.len(), 1, "still one preset, not three");
        assert_eq!(listed[0].description, "louder");
        assert!(load(&dir, "dArK").is_ok());

        remove(&dir, "dark").unwrap();
        assert!(list(&dir).is_empty());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn saving_over_a_preset_needs_saying_so() {
        let dir = scratch("overwrite");
        save(&dir, "Mine", "first", &defaults(), false).unwrap();

        let error = save(&dir, "Mine", "second", &defaults(), false).unwrap_err();
        assert!(matches!(error, PresetError::Exists(_)), "{error}");
        assert_eq!(load(&dir, "Mine").unwrap().description, "first");

        save(&dir, "Mine", "second", &defaults(), true).unwrap();
        assert_eq!(load(&dir, "Mine").unwrap().description, "second");
        std::fs::remove_dir_all(&dir).ok();
    }

    /// A preset nobody can see is worse than one that admits it is broken: the
    /// file is still on disk and the user has no way to learn that.
    #[test]
    fn an_unreadable_preset_is_listed_saying_so() {
        let dir = scratch("broken");
        std::fs::write(dir.join("Bent.json"), "{ not json").unwrap();
        std::fs::write(dir.join("notes.txt"), "ignored").unwrap();
        save(&dir, "Fine", "", &defaults(), false).unwrap();

        let listed = list(&dir);
        assert_eq!(
            listed.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(),
            vec!["Bent", "Fine"],
            "sorted, and the text file is not a preset"
        );
        assert!(listed[0].problem.is_some());
        assert_eq!(listed[1].problem, None);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_missing_folder_is_no_presets_rather_than_an_error() {
        let dir = std::env::temp_dir().join("bw-preset-nothing-here");
        std::fs::remove_dir_all(&dir).ok();
        assert!(list(&dir).is_empty());
        assert!(matches!(load(&dir, "any"), Err(PresetError::Missing(_))));
    }

    #[test]
    fn the_card_shows_the_wallpaper_the_preset_would_set() {
        let dir = scratch("wallpaper");
        let mut config = Config::default();
        config.background.wallpaper_path = r"C:\Users\me\Pictures\night.png".into();
        save(
            &dir,
            "Night",
            "",
            &serde_json::to_value(&config).unwrap(),
            false,
        )
        .unwrap();

        assert_eq!(list(&dir)[0].wallpaper, r"C:\Users\me\Pictures\night.png");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn comparing_a_preset_with_itself_finds_nothing() {
        let current = defaults();
        let comparison = compare(&current, &current);
        assert!(comparison.changes.is_empty());
        assert!(comparison.unknown.is_empty());
    }

    #[test]
    fn a_change_carries_both_sides_as_they_are_shown() {
        let current = defaults();
        let mut other = Config::default();
        other.bar.height = 44;
        other.bar.style = "float".into();
        other.appearance.transparency.enable = !other.appearance.transparency.enable;
        let preset = serde_json::to_value(&other).unwrap();

        let comparison = compare(&current, &preset);
        let paths: Vec<&str> = comparison
            .changes
            .iter()
            .map(|change| change.path.as_str())
            .collect();
        assert!(paths.contains(&"bar.height"), "{paths:?}");
        assert!(paths.contains(&"bar.style"), "{paths:?}");
        assert!(
            paths.contains(&"appearance.transparency.enable"),
            "{paths:?}"
        );

        let height = comparison
            .changes
            .iter()
            .find(|change| change.path == "bar.height")
            .unwrap();
        assert_eq!(height.to, "44");
        let style = comparison
            .changes
            .iter()
            .find(|change| change.path == "bar.style")
            .unwrap();
        // Strings are shown bare rather than quoted.
        assert_eq!(style.to, "float");
    }

    /// The whole reason applying is not a merge: a key from a newer build,
    /// written in, produces a config this build refuses to load.
    #[test]
    fn a_key_this_build_does_not_have_is_reported_not_applied() {
        let current = defaults();
        let mut preset = defaults();
        preset["bar"]["somethingNewer"] = Value::Bool(true);
        preset["wholeNewSection"] = serde_json::json!({ "enable": true });

        let comparison = compare(&current, &preset);
        assert_eq!(
            comparison.unknown,
            vec![
                "bar.somethingNewer".to_owned(),
                "wholeNewSection.enable".to_owned()
            ]
        );
        assert!(
            !comparison
                .changes
                .iter()
                .any(|change| change.path.starts_with("wholeNewSection")),
            "an unknown key is never offered as a change"
        );

        // And applying what was offered still leaves a readable config.
        let mut applied = current.clone();
        let paths: Vec<String> = comparison
            .changes
            .iter()
            .map(|change| change.path.clone())
            .collect();
        apply(&mut applied, &preset, &paths).unwrap();
        serde_json::from_value::<Config>(applied).expect("still a config this build can read");
    }

    /// A preset from an older build must not take away settings it predates.
    #[test]
    fn a_key_the_preset_leaves_out_keeps_its_current_value() {
        let mut current = Config::default();
        current.bar.height = 44;
        let current = serde_json::to_value(&current).unwrap();

        let preset = serde_json::json!({ "bar": { "style": "float" } });
        let comparison = compare(&current, &preset);
        assert_eq!(
            comparison
                .changes
                .iter()
                .map(|change| change.path.as_str())
                .collect::<Vec<_>>(),
            vec!["bar.style"]
        );

        let mut applied = current.clone();
        apply(&mut applied, &preset, &["bar.style".to_owned()]).unwrap();
        assert_eq!(config::get_path(&applied, "bar.height"), Some(&44.into()));
    }

    /// Ticking half the list applies half the list — the same code path as
    /// applying all of it, so there is only one merge to be wrong.
    #[test]
    fn only_the_chosen_paths_are_written() {
        let current = defaults();
        let mut other = Config::default();
        other.bar.height = 44;
        other.bar.style = "float".into();
        let preset = serde_json::to_value(&other).unwrap();

        let mut applied = current.clone();
        let written = apply(&mut applied, &preset, &["bar.style".to_owned()]).unwrap();

        assert_eq!(written, 1);
        assert_eq!(
            config::get_path(&applied, "bar.style"),
            Some(&Value::String("float".into()))
        );
        assert_eq!(
            config::get_path(&applied, "bar.height"),
            config::get_path(&current, "bar.height"),
            "an unticked change is not written"
        );
    }

    #[test]
    fn applying_a_path_that_is_not_in_the_preset_is_refused() {
        let mut current = defaults();
        let preset = serde_json::json!({ "bar": { "style": "float" } });
        assert!(matches!(
            apply(&mut current, &preset, &["bar.height".to_owned()]),
            Err(config::ConfigError::UnknownPath(_))
        ));
    }

    /// A list is replaced whole. Merging `bar.left` element by element would
    /// produce an arrangement of widgets nobody chose.
    #[test]
    fn a_list_is_one_change_rather_than_one_per_element() {
        let current = defaults();
        let preset = serde_json::json!({ "bar": { "left": ["clock"] } });

        let comparison = compare(&current, &preset);
        assert_eq!(
            comparison
                .changes
                .iter()
                .map(|change| change.path.as_str())
                .collect::<Vec<_>>(),
            vec!["bar.left"]
        );

        let mut applied = current.clone();
        apply(&mut applied, &preset, &["bar.left".to_owned()]).unwrap();
        assert_eq!(
            config::get_path(&applied, "bar.left"),
            Some(&serde_json::json!(["clock"]))
        );
    }

    /// The one null in the schema is an unset accent colour, and a preset that
    /// sets it — or clears it — has to come through as a change either way.
    #[test]
    fn an_unset_optional_compares_both_directions() {
        let current = defaults();
        let preset = serde_json::json!({
            "appearance": { "palette": { "accentColor": "#ff0000" } }
        });

        let comparison = compare(&current, &preset);
        let change = comparison
            .changes
            .iter()
            .find(|change| change.path == "appearance.palette.accentColor")
            .expect("setting it is a change");
        assert_eq!(change.from, "", "unset shows as nothing");
        assert_eq!(change.to, "#ff0000");

        let mut set = current.clone();
        apply(&mut set, &preset, std::slice::from_ref(&change.path)).unwrap();
        let back = compare(&set, &current);
        assert!(
            back.changes
                .iter()
                .any(|change| change.path == "appearance.palette.accentColor"),
            "and clearing it again is a change too"
        );
    }
}
