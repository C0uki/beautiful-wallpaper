//! `bw.exe <target> <function> [arguments]`.
//!
//! end4-pC exposes its panels through Quickshell IPC targets, and its keybinds
//! and scripts drive the shell that way. The same vocabulary is kept here, with
//! Tauri's single-instance plugin standing in for the Quickshell socket: running
//! `bw.exe` while the shell is up forwards the arguments to it.

use tauri::{AppHandle, Emitter, Manager};

use crate::commands::event;
use crate::services;
use crate::state::AppState;

const USAGE: &str = "\
beautiful-wallpaper — a Material 3 desktop shell for Windows

  bw                                     start the shell
  bw wallpapers apply <path>             set the wallpaper
  bw wallpapers random                   pick another from the same folder
  bw wallpaperSelector toggle|open|close the wallpaper picker
  bw background toggleWidgets            toggle desktop widget edit mode
  bw config set <a.b.c> <value>          change one setting
  bw config get <a.b.c>                  print one setting
  bw --help                              this message
";

/// Runs a CLI request in a process where no shell is running.
///
/// Only the requests that make sense without a live shell are handled here;
/// everything else needs the running instance, which the single-instance plugin
/// forwards to.
pub fn run(arguments: &[String]) -> i32 {
    match arguments.first().map(String::as_str) {
        Some("--help" | "-h" | "help") => {
            println!("{USAGE}");
            0
        }
        Some("--version" | "-V") => {
            println!("beautiful-wallpaper {}", crate::version());
            0
        }
        Some("config") => match handle_config_offline(&arguments[1..]) {
            Ok(Some(output)) => {
                println!("{output}");
                0
            }
            Ok(None) => 0,
            Err(error) => {
                eprintln!("{error}");
                1
            }
        },
        // The shell is not running, so there is nothing to talk to. Starting it
        // and replaying the request would be surprising, so say so instead.
        Some(_) => {
            eprintln!("beautiful-wallpaper is not running");
            1
        }
        None => 0,
    }
}

/// Handles a request forwarded from a second launch, against the live shell.
pub fn dispatch(app: &AppHandle, arguments: &[String]) -> Result<(), String> {
    let state = app.state::<AppState>();
    let target = arguments.first().map(String::as_str).unwrap_or_default();
    let function = arguments.get(1).map(String::as_str).unwrap_or_default();
    let rest = arguments.get(2..).unwrap_or_default();

    match (target, function) {
        ("wallpapers", "apply") => {
            let path = rest
                .first()
                .ok_or_else(|| "`wallpapers apply` needs a path".to_owned())?;
            services::wallpaper::apply(&state, path)?;
            let theme = services::theme::regenerate(&state)?;
            let _ = app.emit(event::THEME_CHANGED, &theme);
            let _ = app.emit(event::CONFIG_CHANGED, &state.config());
            Ok(())
        }
        ("wallpapers", "random") => {
            services::wallpaper::random(&state)?;
            let theme = services::theme::regenerate(&state)?;
            let _ = app.emit(event::THEME_CHANGED, &theme);
            let _ = app.emit(event::CONFIG_CHANGED, &state.config());
            Ok(())
        }
        ("wallpaperSelector", action) => toggle_surface(app, "wallpaperSelectorOpen", action),
        ("background", "toggleWidgets") => toggle_surface(app, "widgetEditMode", "toggle"),
        ("config", "set") => {
            let (path, value) = (
                rest.first()
                    .ok_or_else(|| "`config set` needs a key".to_owned())?,
                rest.get(1)
                    .ok_or_else(|| "`config set` needs a value".to_owned())?,
            );
            // Accept both `true` and `"true"`: a shell caller cannot easily
            // produce JSON, and the config layer coerces to the stored type.
            let parsed = serde_json::from_str(value)
                .unwrap_or_else(|_| serde_json::Value::String(value.clone()));
            let updated = state
                .set_config_value(path, parsed)
                .map_err(|error| error.to_string())?;
            let _ = app.emit(event::CONFIG_CHANGED, &updated);
            Ok(())
        }
        _ => Err(format!("no IPC target `{target} {function}`")),
    }
}

fn toggle_surface(app: &AppHandle, flag: &str, action: &str) -> Result<(), String> {
    let state = app.state::<AppState>();
    let states = match action {
        "open" => state.set_state(flag, true),
        "close" => state.set_state(flag, false),
        "toggle" | "" => state.toggle_state(flag),
        other => return Err(format!("`{other}` is not open, close or toggle")),
    }
    .ok_or_else(|| format!("there is no surface flag called `{flag}`"))?;

    let _ = app.emit(event::STATE_CHANGED, &states);
    Ok(())
}

/// `config get`/`set` work without a running shell, straight against the file.
fn handle_config_offline(arguments: &[String]) -> Result<Option<String>, String> {
    let path = bw_core::paths::config_file();
    let config = bw_core::config::load(&path).map_err(|error| error.to_string())?;
    let mut json = serde_json::to_value(&config).expect("config is serialisable");

    match arguments.first().map(String::as_str) {
        Some("get") => {
            let key = arguments
                .get(1)
                .ok_or_else(|| "`config get` needs a key".to_owned())?;
            let value = bw_core::config::get_path(&json, key)
                .ok_or_else(|| format!("no config key at `{key}`"))?;
            Ok(Some(
                serde_json::to_string_pretty(value).expect("a config value is serialisable"),
            ))
        }
        Some("set") => {
            let (key, raw) = (
                arguments
                    .get(1)
                    .ok_or_else(|| "`config set` needs a key".to_owned())?,
                arguments
                    .get(2)
                    .ok_or_else(|| "`config set` needs a value".to_owned())?,
            );
            let parsed = serde_json::from_str(raw)
                .unwrap_or_else(|_| serde_json::Value::String(raw.clone()));
            bw_core::config::set_path(&mut json, key, parsed).map_err(|error| error.to_string())?;

            let updated: bw_core::Config =
                serde_json::from_value(json).map_err(|error| error.to_string())?;
            bw_core::config::save(&path, &updated).map_err(|error| error.to_string())?;
            Ok(None)
        }
        _ => Err(USAGE.to_owned()),
    }
}
