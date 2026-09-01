//! Which key combinations Windows keeps for itself, and what to use instead.
//!
//! The original does not need any of this. Hyprland owns the keyboard, every
//! combination is available, and a keybind in its config runs `qs ipc call`.
//! Windows has no such layer: the shell registers system-wide hotkeys itself,
//! and a large part of the `Win`+letter space is already spoken for.
//!
//! Two failures follow, and both are silent. A chord Windows keeps is refused
//! at registration — the key simply does nothing, with no reason given and no
//! reason to look in the config file. And two bindings on the *same* chord are
//! not refused by anything at all: the first one registers, the second one is
//! turned away, and whichever lost is a feature the user cannot reach and
//! cannot explain.
//!
//! So the combinations Windows documents as its own are written down here, and
//! a test holds the shipped defaults against them. That test found both
//! failures already present: `settings` was on `Win+Shift+S`, which opens the
//! Snipping Tool on every Windows 11 machine, and `shelf` and `widgetEditMode`
//! were both on `Win+Shift+D`.
//!
//! What this table cannot say is whether `RegisterHotKey` will actually refuse
//! a given chord. That is undocumented, and it moves with the Windows version
//! and with whatever else is installed — so the refusals the shell reports at
//! runtime are the authority, and this is what keeps the defaults from walking
//! into a fight nobody needs to have.

/// A combination Windows uses, and what for.
pub struct Taken {
    /// Spelled the way the config spells chords.
    pub chord: &'static str,
    /// Shown to the user, so a refusal explains itself.
    pub used_for: &'static str,
}

/// Combinations Windows documents as its own.
///
/// `Win`+letter is nearly all of it; the `Win+Shift` entries are the handful
/// that are also taken, which is what makes `Win+Shift` the space the defaults
/// live in. Windows 10 and 11 differ on a few — both are listed, since the
/// shell supports both and a default has to avoid the union.
const TAKEN: &[Taken] = &[
    Taken {
        chord: "Super+A",
        used_for: "Quick settings",
    },
    Taken {
        chord: "Super+B",
        used_for: "the notification area",
    },
    Taken {
        chord: "Super+C",
        used_for: "Copilot",
    },
    Taken {
        chord: "Super+D",
        used_for: "Show desktop",
    },
    Taken {
        chord: "Super+E",
        used_for: "File Explorer",
    },
    Taken {
        chord: "Super+F",
        used_for: "Feedback Hub",
    },
    Taken {
        chord: "Super+G",
        used_for: "Game Bar",
    },
    Taken {
        chord: "Super+H",
        used_for: "voice typing",
    },
    Taken {
        chord: "Super+I",
        used_for: "Settings",
    },
    Taken {
        chord: "Super+K",
        used_for: "Cast",
    },
    Taken {
        chord: "Super+L",
        used_for: "locking the machine",
    },
    Taken {
        chord: "Super+M",
        used_for: "minimising every window",
    },
    Taken {
        chord: "Super+N",
        used_for: "notifications",
    },
    Taken {
        chord: "Super+O",
        used_for: "locking the orientation",
    },
    Taken {
        chord: "Super+P",
        used_for: "the display mode",
    },
    Taken {
        chord: "Super+Q",
        used_for: "search",
    },
    Taken {
        chord: "Super+R",
        used_for: "Run",
    },
    Taken {
        chord: "Super+S",
        used_for: "search",
    },
    Taken {
        chord: "Super+T",
        used_for: "cycling the taskbar",
    },
    Taken {
        chord: "Super+U",
        used_for: "Accessibility settings",
    },
    Taken {
        chord: "Super+V",
        used_for: "clipboard history",
    },
    Taken {
        chord: "Super+W",
        used_for: "Widgets",
    },
    Taken {
        chord: "Super+X",
        used_for: "the Quick Link menu",
    },
    Taken {
        chord: "Super+Y",
        used_for: "Mixed Reality",
    },
    Taken {
        chord: "Super+Z",
        used_for: "snap layouts",
    },
    Taken {
        chord: "Super+Space",
        used_for: "switching keyboard layout",
    },
    Taken {
        chord: "Super+Tab",
        used_for: "Task View",
    },
    Taken {
        chord: "Super+Comma",
        used_for: "peeking at the desktop",
    },
    Taken {
        chord: "Super+Period",
        used_for: "the emoji panel",
    },
    Taken {
        chord: "Super+Semicolon",
        used_for: "the emoji panel",
    },
    Taken {
        chord: "Super+Pause",
        used_for: "System properties",
    },
    Taken {
        chord: "Super+PrintScreen",
        used_for: "saving a screenshot",
    },
    // The `Win+Shift` combinations that are also taken. Everything else in
    // that space is free, which is why the shell's own keys live there.
    Taken {
        chord: "Super+Shift+S",
        used_for: "the Snipping Tool",
    },
    Taken {
        chord: "Super+Shift+M",
        used_for: "restoring minimised windows",
    },
    Taken {
        chord: "Super+Shift+V",
        used_for: "cycling notifications",
    },
    Taken {
        chord: "Super+Shift+Left",
        used_for: "moving a window to the next monitor",
    },
    Taken {
        chord: "Super+Shift+Right",
        used_for: "moving a window to the next monitor",
    },
    Taken {
        chord: "Super+Shift+Up",
        used_for: "stretching a window",
    },
    Taken {
        chord: "Super+Shift+Down",
        used_for: "restoring a window",
    },
];

/// Modifiers, in the order a canonical chord writes them.
const MODIFIERS: &[(&str, &[&str])] = &[
    (
        "Ctrl",
        &["ctrl", "control", "commandorcontrol", "cmdorctrl"],
    ),
    ("Alt", &["alt", "option"]),
    ("Shift", &["shift"]),
    (
        "Super",
        &["super", "win", "windows", "meta", "cmd", "command"],
    ),
];

/// Rewrites a chord so two spellings of one combination compare equal.
///
/// `win+shift+s`, `Shift+Super+S` and `Super+Shift+S` are the same key to
/// Windows and have to be the same string here, or the duplicate check misses
/// exactly the collision it exists to find.
pub fn normalise(chord: &str) -> String {
    let (modifiers, key) = parse(chord);
    let mut written = modifiers;
    if key.is_empty() {
        return written.join("+");
    }
    written.push(&key);
    written.join("+")
}

/// Splits a chord into its modifiers, in the table's order, and its key.
///
/// The key is returned separately rather than dug back out of the normalised
/// string: `Ctrl+Alt` has no key, and taking the last segment of it would hand
/// back `Alt` — a modifier offered as something to bind.
fn parse(chord: &str) -> (Vec<&'static str>, String) {
    let mut modifiers: Vec<&'static str> = Vec::new();
    let mut key = String::new();

    for part in chord.split('+') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let lower = part.to_ascii_lowercase();

        match MODIFIERS
            .iter()
            .find(|(_, spellings)| spellings.contains(&lower.as_str()))
        {
            Some((canonical, _)) if !modifiers.contains(canonical) => modifiers.push(canonical),
            Some(_) => {}
            // The last non-modifier wins; a chord has one key.
            None => key = capitalise(&lower),
        }
    }

    // Ordered by the table rather than as written, so `Shift+Super+S` and
    // `Super+Shift+S` land on the same string.
    let ordered = MODIFIERS
        .iter()
        .filter(|(canonical, _)| modifiers.contains(canonical))
        .map(|(canonical, _)| *canonical)
        .collect();

    (ordered, key)
}

/// `printscreen` → `PrintScreen`, `s` → `S`.
///
/// Only the first letter: key names arrive from a config people type by hand,
/// and `Printscreen` is what a plain capitalisation would produce for a name
/// Windows writes as one word.
fn capitalise(lower: &str) -> String {
    const NAMES: &[&str] = &[
        "PrintScreen",
        "PageUp",
        "PageDown",
        "CapsLock",
        "NumLock",
        "ScrollLock",
        "ArrowLeft",
        "ArrowRight",
        "ArrowUp",
        "ArrowDown",
    ];
    if let Some(name) = NAMES.iter().find(|name| name.to_ascii_lowercase() == lower) {
        return (*name).to_owned();
    }

    let mut characters = lower.chars();
    match characters.next() {
        Some(first) => first.to_uppercase().chain(characters).collect(),
        None => String::new(),
    }
}

/// What Windows uses this combination for, if it uses it.
pub fn taken_by_windows(chord: &str) -> Option<&'static str> {
    let wanted = normalise(chord);
    TAKEN
        .iter()
        .find(|entry| normalise(entry.chord) == wanted)
        .map(|entry| entry.used_for)
}

/// A combination to offer when one is refused or already taken.
///
/// `Ctrl+Alt` first, because Windows documents nothing there beyond
/// `Ctrl+Alt+Del`, which no application can register anyway. `taken` is
/// whatever the rest of the config is already using, so a suggestion never
/// collides with another binding — which is the failure that produced this
/// module.
pub fn suggest(chord: &str, taken: &[String]) -> Option<String> {
    let (_, key) = parse(chord);
    if key.is_empty() {
        return None;
    }
    let canonical = normalise(chord);

    let used: Vec<String> = taken.iter().map(|found| normalise(found)).collect();

    ["Ctrl+Alt", "Ctrl+Alt+Shift", "Ctrl+Shift"]
        .into_iter()
        .map(|modifiers| normalise(&format!("{modifiers}+{key}")))
        .find(|candidate| {
            *candidate != canonical
                && taken_by_windows(candidate).is_none()
                && !used.contains(candidate)
        })
}

/// One binding, and everything known to be wrong with it.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct KeyStatus {
    /// The config key — `settings`, `captureRegion`.
    pub binding: String,
    pub chord: String,
    /// What Windows documents this combination for, when it documents one.
    /// Known before the key is ever pressed, unlike a refusal.
    pub taken_by_windows: Option<String>,
    /// Other bindings on the same chord. Only one of them can ever work.
    pub shared_with: Vec<String>,
    /// Whether Windows turned the registration down when the shell last
    /// tried. The authority, where the table above is only a warning.
    pub refused: bool,
    /// A free combination to offer instead, when anything is wrong.
    pub suggestion: Option<String>,
}

impl KeyStatus {
    /// Whether this binding needs the user's attention.
    pub fn is_trouble(&self) -> bool {
        self.refused || self.taken_by_windows.is_some() || !self.shared_with.is_empty()
    }
}

/// Every binding with what is wrong with it, for the first-run screen.
///
/// `refused` is the binding names the last registration could not take.
pub fn report(keybinds: &serde_json::Value, refused: &[String]) -> Vec<KeyStatus> {
    let all = chords(keybinds);
    let taken: Vec<String> = all.iter().map(|(_, chord)| chord.clone()).collect();
    let shared = collisions(keybinds);

    all.iter()
        .map(|(binding, chord)| {
            let canonical = normalise(chord);
            let shared_with: Vec<String> = shared
                .iter()
                .find(|(found, _)| *found == canonical)
                .map(|(_, bindings)| {
                    bindings
                        .iter()
                        .filter(|found| *found != binding)
                        .cloned()
                        .collect()
                })
                .unwrap_or_default();

            let mut status = KeyStatus {
                binding: binding.clone(),
                chord: chord.clone(),
                taken_by_windows: taken_by_windows(chord).map(str::to_owned),
                shared_with,
                refused: refused.contains(binding),
                suggestion: None,
            };
            if status.is_trouble() {
                status.suggestion = suggest(chord, &taken);
            }
            status
        })
        .collect()
}

/// Every chord in a `keybinds` object, as `(binding, chord)`.
///
/// Read out of the serialised config rather than field by field, so a key
/// added to the schema is checked without anybody remembering to add a line.
pub fn chords(keybinds: &serde_json::Value) -> Vec<(String, String)> {
    let Some(fields) = keybinds.as_object() else {
        return Vec::new();
    };
    fields
        .iter()
        .filter_map(|(name, value)| {
            let chord = value.as_str()?.trim().to_owned();
            (!chord.is_empty()).then_some((name.clone(), chord))
        })
        .collect()
}

/// Bindings sharing one chord, as `(chord, the bindings on it)`.
///
/// Nothing refuses this: the first registration wins and the second is turned
/// away, so whichever lost is a feature that cannot be reached and cannot be
/// explained.
pub fn collisions(keybinds: &serde_json::Value) -> Vec<(String, Vec<String>)> {
    let mut seen: Vec<(String, Vec<String>)> = Vec::new();

    for (binding, chord) in chords(keybinds) {
        let canonical = normalise(&chord);
        match seen.iter_mut().find(|(found, _)| *found == canonical) {
            Some((_, bindings)) => bindings.push(binding),
            None => seen.push((canonical, vec![binding])),
        }
    }

    seen.retain(|(_, bindings)| bindings.len() > 1);
    seen
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Config;

    fn defaults() -> serde_json::Value {
        serde_json::to_value(Config::default().keybinds).expect("keybinds are serialisable")
    }

    #[test]
    fn one_combination_has_one_spelling() {
        assert_eq!(normalise("win+shift+s"), "Shift+Super+S");
        assert_eq!(normalise("Shift+Super+S"), "Shift+Super+S");
        assert_eq!(normalise("Super+Shift+S"), "Shift+Super+S");
        assert_eq!(normalise("META+SHIFT+s"), "Shift+Super+S");
        assert_eq!(normalise("Alt+Space"), "Alt+Space");
        assert_eq!(normalise("Print"), "Print");
        assert_eq!(normalise("ctrl+printscreen"), "Ctrl+PrintScreen");
    }

    /// A duplicated modifier is a typo, not a different chord.
    #[test]
    fn a_repeated_modifier_is_written_once() {
        assert_eq!(normalise("Ctrl+Ctrl+S"), "Ctrl+S");
        assert_eq!(normalise("Super+Win+X"), "Super+X");
    }

    #[test]
    fn a_chord_windows_keeps_says_what_for() {
        assert_eq!(taken_by_windows("Super+I"), Some("Settings"));
        assert_eq!(taken_by_windows("win+shift+s"), Some("the Snipping Tool"));
        assert_eq!(
            taken_by_windows("Super+Shift+M"),
            Some("restoring minimised windows")
        );
        assert_eq!(taken_by_windows("Super+Shift+A"), None);
        assert_eq!(taken_by_windows("Ctrl+Alt+S"), None);
    }

    /// The whole point of the shipped defaults living in `Win+Shift`: a
    /// default on a combination Windows already uses is a key that does
    /// nothing, and the user has no way to find out why.
    #[test]
    fn no_shipped_default_fights_windows() {
        let clashing: Vec<String> = chords(&defaults())
            .into_iter()
            .filter_map(|(binding, chord)| {
                taken_by_windows(&chord).map(|used_for| {
                    format!("{binding} is on {chord}, which Windows uses for {used_for}")
                })
            })
            .collect();

        assert!(clashing.is_empty(), "{clashing:#?}");
    }

    /// Nothing refuses two bindings on one chord, which is what makes it worth
    /// a test: the second registration is turned away and that feature is
    /// simply gone.
    #[test]
    fn no_two_defaults_share_a_chord() {
        assert!(
            collisions(&defaults()).is_empty(),
            "{:#?}",
            collisions(&defaults())
        );
    }

    #[test]
    fn collisions_name_every_binding_on_the_chord() {
        let keybinds = serde_json::json!({
            "shelf": "Super+Shift+D",
            "widgetEditMode": "super+shift+d",
            "overlay": "Super+Shift+O",
            "enable": true,
        });

        let found = collisions(&keybinds);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].0, "Shift+Super+D");
        assert_eq!(
            found[0].1,
            vec!["shelf".to_owned(), "widgetEditMode".to_owned()]
        );
    }

    #[test]
    fn a_suggestion_avoids_windows_and_the_rest_of_the_config() {
        let suggestion = suggest("Super+Shift+S", &[]).unwrap();
        assert_eq!(suggestion, "Ctrl+Alt+S");
        assert!(taken_by_windows(&suggestion).is_none());

        // Already used by something else, so the next rung of the ladder.
        let suggestion = suggest("Super+Shift+S", &["Ctrl+Alt+S".to_owned()]).unwrap();
        assert_eq!(suggestion, "Ctrl+Alt+Shift+S");

        // And the spelling of what is taken does not matter.
        let suggestion = suggest("Super+Shift+S", &["ctrl+alt+s".to_owned()]).unwrap();
        assert_eq!(suggestion, "Ctrl+Alt+Shift+S");
    }

    #[test]
    fn a_suggestion_keeps_the_key_that_was_asked_for() {
        assert_eq!(suggest("Print", &[]).unwrap(), "Ctrl+Alt+Print");
        assert_eq!(suggest("Alt+Space", &[]).unwrap(), "Ctrl+Alt+Space");
    }

    #[test]
    fn there_is_no_suggestion_when_every_rung_is_taken() {
        let taken: Vec<String> = ["Ctrl+Alt+S", "Ctrl+Alt+Shift+S", "Ctrl+Shift+S"]
            .iter()
            .map(|chord| (*chord).to_owned())
            .collect();
        assert_eq!(suggest("Super+Shift+S", &taken), None);
    }

    #[test]
    fn a_chord_with_no_key_suggests_nothing() {
        assert_eq!(suggest("Ctrl+Alt", &[]), None);
        assert_eq!(suggest("", &[]), None);
    }

    /// `enable` is a boolean in the same object, and reading it as a chord
    /// would put "true" in the list.
    /// Nothing is wrong with the shipped defaults, which is the state the
    /// first-run screen should find on a machine that gives up every key.
    #[test]
    fn the_defaults_report_nothing_to_fix() {
        let found = report(&defaults(), &[]);
        assert!(found.len() > 8, "every binding is reported");
        let trouble: Vec<&KeyStatus> = found.iter().filter(|key| key.is_trouble()).collect();
        assert!(trouble.is_empty(), "{trouble:#?}");
        assert!(found.iter().all(|key| key.suggestion.is_none()));
    }

    #[test]
    fn a_refusal_and_a_collision_both_come_with_a_way_out() {
        let keybinds = serde_json::json!({
            "settings": "Super+Shift+S",
            "shelf": "Super+Shift+D",
            "widgetEditMode": "Super+Shift+D",
            "overlay": "Super+Shift+O",
        });
        let found = report(&keybinds, &["overlay".to_owned()]);
        let of = |binding: &str| {
            found
                .iter()
                .find(|key| key.binding == binding)
                .unwrap_or_else(|| panic!("no {binding}"))
                .clone()
        };

        // Known before it is pressed, from the table.
        let settings = of("settings");
        assert_eq!(
            settings.taken_by_windows.as_deref(),
            Some("the Snipping Tool")
        );
        assert!(settings.suggestion.is_some());

        // Known only because two bindings name one chord.
        let shelf = of("shelf");
        assert_eq!(shelf.shared_with, vec!["widgetEditMode".to_owned()]);
        assert!(shelf.taken_by_windows.is_none());
        assert!(shelf.suggestion.is_some());

        // Known only because Windows said no at registration.
        let overlay = of("overlay");
        assert!(overlay.refused);
        assert!(overlay.taken_by_windows.is_none());
        assert!(overlay.suggestion.is_some());

        // And a binding with nothing wrong offers nothing.
        assert!(!of("widgetEditMode").refused);
        assert_eq!(of("widgetEditMode").shared_with, vec!["shelf".to_owned()]);
    }

    #[test]
    fn only_the_string_fields_are_chords() {
        let found = chords(&defaults());
        assert!(!found.iter().any(|(binding, _)| binding == "enable"));
        assert!(found.iter().any(|(binding, _)| binding == "overview"));
    }
}
