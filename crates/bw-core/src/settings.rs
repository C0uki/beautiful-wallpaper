//! Describing the config so a settings screen can be built from it.
//!
//! The original hand-writes a control per key — six thousand lines of QML
//! across ten pages. That is a reasonable thing to do once, and a bad thing to
//! maintain: every key added to the schema is a key somebody has to remember
//! to add a control for, and the failure when they forget is silent. The
//! setting is simply not there, and the only way to notice is to go looking
//! for it.
//!
//! So the shape of the form is derived from the schema instead. Everything
//! here comes from walking [`Config::default`], which means a new config key
//! has a control the moment it exists, with the right type and the right
//! default. What the schema cannot say — which page a section belongs on, that
//! a number is really a percentage, what a string is choosing between — is
//! supplied by the frontend on top of this.
//!
//! The one thing this deliberately does *not* derive is wording. Doc comments
//! are developer prose in English; UI copy is neither, and it has to be
//! translatable. Labels here are mechanical (`reserveSpace` → "Reserve
//! space"), and anything that reads badly is overridden where the translations
//! live.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use ts_rs::TS;

use crate::Config;

/// What kind of control a value wants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum FieldKind {
    Toggle,
    /// A whole number.
    Integer,
    /// A number with a fraction — an opacity, a scale, a fraction of a screen.
    Decimal,
    Text,
    /// A list of strings, edited as lines.
    TextList,
    /// Something the generated form cannot edit — a list of objects, say.
    ///
    /// Listed rather than dropped: a setting nobody can see is worse than one
    /// that says it has to be edited in the file, because only the second one
    /// tells you it exists.
    Unsupported,
}

/// One editable value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Field {
    /// The dotted path `set_config_value` takes — `bar.reserveSpace`.
    pub path: String,
    /// The top-level section, which is what decides the page it lands on.
    pub section: String,
    /// A mechanical label from the last segment of the path.
    pub label: String,
    /// The enclosing group's label, empty at the top of a section. What turns
    /// a flat list of two hundred rows into something with headings.
    pub group: String,
    pub kind: FieldKind,
}

/// Words that are not words.
///
/// A mechanical label turns `ocrLanguage` into "Ocr language", which looks
/// like a typo every time somebody reads it. Only the ones this schema
/// actually contains are listed; a new acronym needs a line here, and the
/// test below is what makes that obvious.
const ACRONYMS: &[(&str, &str)] = &[
    ("ai", "AI"),
    ("ocr", "OCR"),
    ("ui", "UI"),
    ("osd", "OSD"),
    ("id", "ID"),
    ("url", "URL"),
    ("api", "API"),
    ("cpu", "CPU"),
    ("ram", "RAM"),
    ("fps", "FPS"),
    ("glazewm", "GlazeWM"),
    ("komorebi", "komorebi"),
    ("usc", "USC"),
];

/// A label for one path segment.
///
/// `reserveSpace` → "Reserve space": split on the camel humps, keep acronyms
/// whole, and capitalise only the first word. Sentence case rather than title
/// case, because the rest of the interface is in sentence case.
pub fn label_for(key: &str) -> String {
    let words = split_camel(key);
    let mut out = String::new();

    for (index, word) in words.iter().enumerate() {
        if index > 0 {
            out.push(' ');
        }
        let lower = word.to_ascii_lowercase();
        match ACRONYMS.iter().find(|(from, _)| *from == lower) {
            Some((_, shown)) => out.push_str(shown),
            None if index == 0 => {
                let mut characters = word.chars();
                if let Some(first) = characters.next() {
                    out.extend(first.to_uppercase());
                    out.push_str(&characters.as_str().to_ascii_lowercase());
                }
            }
            None => out.push_str(&lower),
        }
    }

    out
}

/// Splits `reserveSpace` into `["reserve", "Space"]`, keeping digits with the
/// word they follow.
fn split_camel(key: &str) -> Vec<String> {
    let mut words: Vec<String> = Vec::new();
    let mut current = String::new();

    for character in key.chars() {
        if character.is_ascii_uppercase() && !current.is_empty() {
            words.push(std::mem::take(&mut current));
        }
        current.push(character);
    }
    if !current.is_empty() {
        words.push(current);
    }
    if words.is_empty() {
        words.push(key.to_owned());
    }
    words
}

/// Every editable value in the config, in the order the schema declares them.
pub fn fields() -> Vec<Field> {
    from_value(&serde_json::to_value(Config::default()).expect("the config is serialisable"))
}

/// The same, for a config that is not the default — used by the tests.
fn from_value(root: &Value) -> Vec<Field> {
    let mut found = Vec::new();
    let Some(sections) = root.as_object() else {
        return found;
    };

    for (section, body) in sections {
        walk(section, section, "", body, &mut found);
    }
    found
}

fn walk(section: &str, path: &str, group: &str, node: &Value, into: &mut Vec<Field>) {
    if let Value::Object(children) = node {
        // The group a nested value belongs to is its own parent's label, which
        // is what gives a page its headings.
        let label = path.rsplit('.').next().unwrap_or(path);
        let inner = if path == section {
            String::new()
        } else {
            label_for(label)
        };

        for (key, child) in children {
            walk(section, &format!("{path}.{key}"), &inner, child, into);
        }
        return;
    }

    let key = path.rsplit('.').next().unwrap_or(path);
    into.push(Field {
        path: path.to_owned(),
        section: section.to_owned(),
        label: label_for(key),
        group: group.to_owned(),
        kind: kind_of(node),
    });
}

fn kind_of(node: &Value) -> FieldKind {
    match node {
        Value::Bool(_) => FieldKind::Toggle,
        // Whole and fractional numbers get different controls: a step of one
        // on an opacity is useless, and a decimal point on a pixel count is
        // an invitation to type something meaningless.
        Value::Number(number) => {
            if number.is_f64() && number.as_f64().is_some_and(|found| found.fract() != 0.0) {
                FieldKind::Decimal
            } else if number.is_f64() {
                // A float that happens to be whole — `1.0` — is still a float
                // in the schema, and giving it an integer control would make
                // `roundingScale` un-settable to anything but 1 and 2.
                FieldKind::Decimal
            } else {
                FieldKind::Integer
            }
        }
        Value::String(_) => FieldKind::Text,
        // The only nulls in this schema are unset optional strings, and a test
        // holds that true. Treating one as text is what lets it be set.
        Value::Null => FieldKind::Text,
        Value::Array(items) => {
            if items.iter().all(Value::is_string) {
                // An empty list is taken as a list of strings: it is what every
                // empty list in this schema is, and the alternative is telling
                // the user that an editable setting cannot be edited.
                FieldKind::TextList
            } else {
                FieldKind::Unsupported
            }
        }
        Value::Object(_) => FieldKind::Unsupported,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_label_is_the_key_in_sentence_case() {
        assert_eq!(label_for("reserveSpace"), "Reserve space");
        assert_eq!(label_for("enable"), "Enable");
        assert_eq!(label_for("wallpaperAnimation"), "Wallpaper animation");
        assert_eq!(
            label_for("clicklessCornerVerticalOffset"),
            "Clickless corner vertical offset"
        );
    }

    /// "Ocr language" reads like a typo every time somebody sees it.
    #[test]
    fn acronyms_survive_the_labelling() {
        assert_eq!(label_for("ocrLanguage"), "OCR language");
        assert_eq!(label_for("captureOcr"), "Capture OCR");
        assert_eq!(label_for("ai"), "AI");
        assert_eq!(label_for("ui"), "UI");
        assert_eq!(label_for("osd"), "OSD");
        assert_eq!(label_for("glazewm"), "GlazeWM");
        // "Use usc units" reads as a typo; the units are United States
        // Customary, and the screen says so.
        assert_eq!(label_for("useUscUnits"), "Use USC units");
        // A proper name that is deliberately lower case stays that way.
        assert_eq!(label_for("komorebi"), "komorebi");
    }

    #[test]
    fn every_leaf_in_the_schema_becomes_a_field() {
        let fields = fields();
        let value = serde_json::to_value(Config::default()).expect("serialisable");

        fn count(node: &serde_json::Value) -> usize {
            match node {
                serde_json::Value::Object(children) => children.values().map(count).sum(),
                _ => 1,
            }
        }

        assert_eq!(
            fields.len(),
            count(&value),
            "a leaf without a field is a setting nobody can reach"
        );
        assert!(fields.len() > 100, "the schema is not that small");
    }

    #[test]
    fn a_path_is_the_one_set_config_value_takes() {
        let fields = fields();
        let reserve = fields
            .iter()
            .find(|field| field.path == "bar.reserveSpace")
            .expect("the bar reserves space");

        assert_eq!(reserve.section, "bar");
        assert_eq!(reserve.label, "Reserve space");
        assert_eq!(reserve.kind, FieldKind::Toggle);
        assert_eq!(reserve.group, "", "it is at the top of its section");
    }

    /// Two hundred rows in one list is not a settings page; the groups are
    /// what give it headings.
    #[test]
    fn a_nested_value_carries_its_parents_label_as_its_group() {
        let fields = fields();
        let style = fields
            .iter()
            .find(|field| field.path == "sidebar.quickToggles.style")
            .expect("the toggles have a style");

        assert_eq!(style.section, "sidebar");
        assert_eq!(style.group, "Quick toggles");
        assert_eq!(style.label, "Style");
    }

    #[test]
    fn whole_and_fractional_numbers_get_different_controls() {
        let fields = fields();
        let kind = |path: &str| {
            fields
                .iter()
                .find(|field| field.path == path)
                .unwrap_or_else(|| panic!("no {path}"))
                .kind
        };

        assert_eq!(kind("bar.height"), FieldKind::Integer);
        assert_eq!(kind("appearance.roundingScale"), FieldKind::Decimal);
        assert_eq!(kind("sidebar.width"), FieldKind::Decimal);
    }

    #[test]
    fn a_list_of_strings_is_editable_and_a_list_of_objects_is_not() {
        let fields = fields();
        let kind = |path: &str| {
            fields
                .iter()
                .find(|field| field.path == path)
                .unwrap_or_else(|| panic!("no {path}"))
                .kind
        };

        assert_eq!(kind("bar.left"), FieldKind::TextList);
        assert_eq!(kind("dock.pinnedApps"), FieldKind::TextList);
    }

    /// The assumption `kind_of` makes about nulls, stated as a test: if a
    /// non-string optional is ever added, this is what says so.
    #[test]
    fn every_null_in_the_schema_is_an_unset_string() {
        let value = serde_json::to_value(Config::default()).expect("serialisable");
        let mut nulls = Vec::new();

        fn find(node: &serde_json::Value, path: &str, into: &mut Vec<String>) {
            match node {
                serde_json::Value::Object(children) => {
                    for (key, child) in children {
                        let next = if path.is_empty() {
                            key.clone()
                        } else {
                            format!("{path}.{key}")
                        };
                        find(child, &next, into);
                    }
                }
                serde_json::Value::Null => into.push(path.to_owned()),
                _ => {}
            }
        }
        find(&value, "", &mut nulls);

        // Written out rather than counted: adding one should make somebody
        // look at `kind_of` and decide, not quietly bump a number.
        assert_eq!(
            nulls,
            vec!["appearance.palette.accentColor".to_owned()],
            "a new optional needs a decision in `kind_of`"
        );
    }

    #[test]
    fn the_order_is_the_schemas_own() {
        let fields = fields();
        let bar: Vec<&str> = fields
            .iter()
            .filter(|field| field.section == "bar" && field.group.is_empty())
            .map(|field| field.path.as_str())
            .collect();

        // `enable` is declared first and stays first, rather than being sorted
        // into the middle of the page under "e".
        assert_eq!(bar.first(), Some(&"bar.enable"));
    }

    #[test]
    fn every_section_of_the_config_is_represented() {
        let fields = fields();
        let value = serde_json::to_value(Config::default()).expect("serialisable");

        for section in value.as_object().expect("an object").keys() {
            assert!(
                fields.iter().any(|field| &field.section == section),
                "nothing from `{section}` reached the form"
            );
        }
    }

    #[test]
    fn a_key_with_no_humps_still_gets_a_label() {
        assert_eq!(label_for("style"), "Style");
        assert_eq!(label_for(""), "");
    }
}
