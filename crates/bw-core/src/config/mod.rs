//! Loading, saving and patching `config.json`.
//!
//! end4-pC exposes `Config.setNestedValue("a.b.c", v)` to its IPC layer and its
//! launcher, with JSON type coercion so a shell caller can pass `"true"` for a
//! boolean. [`set_path`] reproduces that behaviour.

mod schema;

pub use schema::*;

use serde_json::Value;
use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("could not read {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("could not write {path}: {source}")]
    Write {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("{path} is not valid config JSON: {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("no config key at `{0}`")]
    UnknownPath(String),
    #[error("`{value}` is not a valid {expected} for `{path}`")]
    TypeMismatch {
        path: String,
        expected: &'static str,
        value: String,
    },
}

/// Reads the config, falling back to defaults when the file does not exist yet.
///
/// A file that exists but cannot be parsed is an error rather than a silent
/// reset: overwriting a config the user hand-edited would lose their work.
pub fn load(path: &Path) -> Result<Config, ConfigError> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Config::default())
        }
        Err(source) => {
            return Err(ConfigError::Read {
                path: path.to_owned(),
                source,
            })
        }
    };
    parse(&text).map_err(|source| ConfigError::Parse {
        path: path.to_owned(),
        source,
    })
}

/// Parses config JSON, filling in every key the document leaves out.
pub fn parse(text: &str) -> Result<Config, serde_json::Error> {
    if text.trim().is_empty() {
        return Ok(Config::default());
    }
    serde_json::from_str(text)
}

/// Writes the config back out, creating the parent directory if needed.
///
/// Every key is written, not just the ones that differ from the defaults, so the
/// file doubles as documentation of what can be set — the same choice the
/// original `JsonAdapter` makes.
pub fn save(path: &Path, config: &Config) -> Result<(), ConfigError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|source| ConfigError::Write {
            path: parent.to_owned(),
            source,
        })?;
    }
    let mut text = serde_json::to_string_pretty(config).expect("config is always serialisable");
    text.push('\n');
    std::fs::write(path, text).map_err(|source| ConfigError::Write {
        path: path.to_owned(),
        source,
    })
}

/// Looks up a dotted path such as `background.widgets.clock.enable`.
pub fn get_path<'a>(root: &'a Value, path: &str) -> Option<&'a Value> {
    let mut cursor = root;
    for segment in path.split('.') {
        cursor = match cursor {
            Value::Object(map) => map.get(segment)?,
            Value::Array(items) => items.get(segment.parse::<usize>().ok()?)?,
            _ => return None,
        };
    }
    Some(cursor)
}

/// Assigns to a dotted path, coercing `new` to the type already stored there.
///
/// The path must already exist: this is a typed config, and silently growing it
/// from a typo would produce a key nothing reads.
pub fn set_path(root: &mut Value, path: &str, new: Value) -> Result<(), ConfigError> {
    let coerced = {
        let current = get_path(root, path).ok_or_else(|| ConfigError::UnknownPath(path.into()))?;
        coerce(current, new, path)?
    };

    let (parent_path, leaf) = match path.rsplit_once('.') {
        Some((parent, leaf)) => (parent, leaf),
        None => ("", path),
    };

    let parent = if parent_path.is_empty() {
        root
    } else {
        // Re-walking is fine here: config trees are shallow, and it keeps the
        // borrow checker out of a mutable-cursor loop.
        walk_mut(root, parent_path).ok_or_else(|| ConfigError::UnknownPath(path.into()))?
    };

    match parent {
        Value::Object(map) => {
            map.insert(leaf.to_owned(), coerced);
            Ok(())
        }
        Value::Array(items) => {
            let index = leaf
                .parse::<usize>()
                .map_err(|_| ConfigError::UnknownPath(path.into()))?;
            let slot = items
                .get_mut(index)
                .ok_or_else(|| ConfigError::UnknownPath(path.into()))?;
            *slot = coerced;
            Ok(())
        }
        _ => Err(ConfigError::UnknownPath(path.into())),
    }
}

fn walk_mut<'a>(root: &'a mut Value, path: &str) -> Option<&'a mut Value> {
    let mut cursor = root;
    for segment in path.split('.') {
        cursor = match cursor {
            Value::Object(map) => map.get_mut(segment)?,
            Value::Array(items) => items.get_mut(segment.parse::<usize>().ok()?)?,
            _ => return None,
        };
    }
    Some(cursor)
}

/// Converts a loosely typed incoming value to the type the config already holds,
/// so `bw.exe config set bar.bottom true` works from a shell.
fn coerce(current: &Value, new: Value, path: &str) -> Result<Value, ConfigError> {
    let mismatch = |expected: &'static str, value: &Value| ConfigError::TypeMismatch {
        path: path.to_owned(),
        expected,
        value: value.to_string(),
    };

    Ok(match current {
        Value::Bool(_) => match &new {
            Value::Bool(_) => new,
            Value::String(text) => match text.as_str() {
                "true" | "1" | "yes" => Value::Bool(true),
                "false" | "0" | "no" => Value::Bool(false),
                _ => return Err(mismatch("boolean", &new)),
            },
            Value::Number(number) => Value::Bool(number.as_f64().is_some_and(|n| n != 0.0)),
            _ => return Err(mismatch("boolean", &new)),
        },
        // Integer fields must stay integers: `52.0` fails to deserialise into a
        // `u32`, so parse to the narrowest type that fits before falling back to
        // a float.
        Value::Number(current) => match &new {
            Value::Number(incoming) => match incoming.as_f64() {
                Some(value) if !current.is_f64() && value.fract() == 0.0 => {
                    Value::Number(number_from_f64(value).ok_or_else(|| mismatch("number", &new))?)
                }
                _ => new,
            },
            Value::String(text) => parse_number(text.trim())
                .map(Value::Number)
                .ok_or_else(|| mismatch("number", &new))?,
            _ => return Err(mismatch("number", &new)),
        },
        Value::String(_) => match new {
            Value::String(_) => new,
            other => Value::String(match other {
                Value::Bool(b) => b.to_string(),
                Value::Number(n) => n.to_string(),
                other => return Err(mismatch("string", &other)),
            }),
        },
        Value::Array(_) => match &new {
            Value::Array(_) => new,
            // A comma-separated string is the only ergonomic way to set a list
            // from a command line.
            Value::String(text) => Value::Array(
                text.split(',')
                    .map(|part| Value::String(part.trim().to_owned()))
                    .collect(),
            ),
            _ => return Err(mismatch("array", &new)),
        },
        Value::Object(_) => match &new {
            Value::Object(_) => new,
            _ => return Err(mismatch("object", &new)),
        },
        Value::Null => new,
    })
}

/// Parses a config number, preferring integers so integer fields keep their type.
fn parse_number(text: &str) -> Option<serde_json::Number> {
    if let Ok(value) = text.parse::<i64>() {
        return Some(value.into());
    }
    if let Ok(value) = text.parse::<u64>() {
        return Some(value.into());
    }
    number_from_f64(text.parse::<f64>().ok()?)
}

fn number_from_f64(value: f64) -> Option<serde_json::Number> {
    if value.fract() == 0.0 && value >= i64::MIN as f64 && value <= i64::MAX as f64 {
        return Some((value as i64).into());
    }
    serde_json::Number::from_f64(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn defaults() -> Value {
        serde_json::to_value(Config::default()).unwrap()
    }

    #[test]
    fn defaults_round_trip_through_json() {
        let config = Config::default();
        let text = serde_json::to_string(&config).unwrap();
        assert_eq!(parse(&text).unwrap(), config);
    }

    #[test]
    fn missing_keys_fall_back_to_defaults() {
        let config = parse(r#"{"bar":{"bottom":true}}"#).unwrap();
        assert!(config.bar.bottom);
        assert_eq!(config.bar.height, Bar::default().height);
        assert_eq!(config.appearance, Appearance::default());
    }

    #[test]
    fn empty_document_is_all_defaults() {
        assert_eq!(parse("").unwrap(), Config::default());
        assert_eq!(parse("{}").unwrap(), Config::default());
    }

    #[test]
    fn unknown_keys_are_rejected_rather_than_silently_dropped() {
        let error = parse(r#"{"bar":{"botom":true}}"#).unwrap_err();
        assert!(error.to_string().contains("botom"), "{error}");
    }

    #[test]
    fn get_path_walks_nested_objects() {
        let root = defaults();
        assert_eq!(
            get_path(&root, "background.widgets.clock.id"),
            Some(&Value::String("clock".into()))
        );
        assert!(get_path(&root, "background.widgets.nope").is_none());
    }

    #[test]
    fn set_path_coerces_strings_to_the_stored_type() {
        let mut root = defaults();
        set_path(&mut root, "bar.bottom", Value::String("true".into())).unwrap();
        set_path(&mut root, "bar.height", Value::String("52".into())).unwrap();
        set_path(&mut root, "bar.left", Value::String("media, clock".into())).unwrap();
        set_path(
            &mut root,
            "appearance.roundingScale",
            Value::String("1.25".into()),
        )
        .unwrap();

        let config: Config = serde_json::from_value(root).unwrap();
        assert!(config.bar.bottom);
        assert_eq!(config.bar.height, 52);
        assert_eq!(
            config.bar.left,
            vec!["media".to_owned(), "clock".to_owned()]
        );
        assert_eq!(config.appearance.rounding_scale, 1.25);
    }

    #[test]
    fn integer_fields_stay_integers() {
        let mut root = defaults();
        // A JSON number that arrives as a float must not turn `u32` into `52.0`.
        set_path(&mut root, "bar.height", serde_json::json!(52.0)).unwrap();
        assert_eq!(get_path(&root, "bar.height"), Some(&serde_json::json!(52)));
        serde_json::from_value::<Config>(root).unwrap();
    }

    #[test]
    fn set_path_indexes_into_arrays() {
        let mut root = defaults();
        set_path(&mut root, "bar.left.0", Value::String("clock".into())).unwrap();
        assert_eq!(
            get_path(&root, "bar.left.0"),
            Some(&Value::String("clock".into()))
        );
    }

    #[test]
    fn set_path_rejects_unknown_keys_and_bad_types() {
        let mut root = defaults();
        assert!(matches!(
            set_path(&mut root, "bar.nope", Value::Bool(true)),
            Err(ConfigError::UnknownPath(_))
        ));
        assert!(matches!(
            set_path(&mut root, "bar.height", Value::String("tall".into())),
            Err(ConfigError::TypeMismatch { .. })
        ));
    }

    #[test]
    fn save_then_load_preserves_edits() {
        let dir = std::env::temp_dir().join(format!("bw-config-{}", std::process::id()));
        let path = dir.join("config.json");
        let mut config = Config::default();
        config.background.wallpaper_path = r"C:\Users\me\Pictures\a.png".into();
        config.bar.height = 44;

        save(&path, &config).unwrap();
        assert_eq!(load(&path).unwrap(), config);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_missing_file_loads_as_defaults() {
        let path = std::env::temp_dir().join("bw-config-does-not-exist/config.json");
        assert_eq!(load(&path).unwrap(), Config::default());
    }
}
