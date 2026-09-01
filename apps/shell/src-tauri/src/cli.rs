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
  bw capture region|ocr|translate        pick a region of the screen
  bw session toggle|open|close           the way out of the session
  bw desktopMenu toggle|open|close       the desktop menu, at the pointer
  bw shelf toggle|open|close             the drop shelf
  bw overlay toggle|open|close           the floating overlay
  bw settings toggle|open|close          the settings screen
  bw wizard toggle|open|close            the first-run screen, again
  bw config set <a.b.c> <value>          change one setting
  bw config get <a.b.c>                  print one setting
  bw preset list                         the saved configurations
  bw preset save <name> [description]    save the config under a name
  bw preset apply <name>                 put a saved configuration back
  bw preset remove <name>                delete one
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
        Some("preset") => match handle_preset_offline(&arguments[1..]) {
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
        ("session", action) => toggle_surface(app, "sessionOpen", action),
        ("shelf", action) => toggle_surface(app, "shelfOpen", action),
        ("overlay", action) => toggle_surface(app, "overlayOpen", action),
        ("settings", action) => toggle_surface(app, "settingsOpen", action),
        ("wizard", action) => toggle_surface(app, "wizardOpen", action),
        // Not `toggle_surface`: the menu opens where the pointer is, so the
        // anchor has to be taken before the surface is shown.
        ("desktopMenu", action) => crate::commands::toggle_desktop_menu(
            app.clone(),
            app.state::<AppState>(),
            app.state::<crate::state::DesktopMenuHandle>(),
            Some(action.to_owned()),
        ),
        ("background", "toggleWidgets") => toggle_surface(app, "widgetEditMode", "toggle"),
        ("capture", mode) => {
            let mode = match mode {
                "region" | "screenshot" | "" => bw_core::capture::CaptureMode::Screenshot,
                "ocr" | "text" => bw_core::capture::CaptureMode::Ocr,
                "translate" => bw_core::capture::CaptureMode::Translate,
                other => return Err(format!("`{other}` is not region, ocr or translate")),
            };
            crate::commands::start_capture(
                app.clone(),
                app.state::<AppState>(),
                app.state::<crate::state::CaptureHandle>(),
                mode,
            )
        }
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
        ("preset", "list") => {
            for summary in services::preset::list() {
                println!("{}", summary.name);
            }
            Ok(())
        }
        ("preset", "save") => {
            let name = rest
                .first()
                .ok_or_else(|| "`preset save` needs a name".to_owned())?;
            let description = rest.get(1..).unwrap_or_default().join(" ");
            // Overwriting: the confirmation the settings screen asks for has
            // nowhere to happen on a command line, and a caller that typed the
            // name twice meant it.
            services::preset::save(&state, name, &description, true)
                .map(|_| ())
                .map_err(|error| error.to_string())
        }
        ("preset", "apply") => {
            let name = rest
                .first()
                .ok_or_else(|| "`preset apply` needs a name".to_owned())?;
            // Everything the preset changes, since there is no list here to
            // untick anything from.
            let paths: Vec<String> = services::preset::compare(&state, name)
                .map_err(|error| error.to_string())?
                .changes
                .into_iter()
                .map(|change| change.path)
                .collect();
            services::preset::apply(app, &state, name, &paths)
                .map(|_| ())
                .map_err(|error| error.to_string())
        }
        ("preset", "remove") => {
            let name = rest
                .first()
                .ok_or_else(|| "`preset remove` needs a name".to_owned())?;
            services::preset::remove(name)
                .map(|_| ())
                .map_err(|error| error.to_string())
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

    crate::surfaces::apply_states(app, &states);
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

/// `preset` works without a running shell: a preset is a file, and applying one
/// is a config edit, both of which the offline path already does.
///
/// What it cannot do is the part that needs a live shell — telling Windows
/// about a new wallpaper, regenerating the palette, re-registering hotkeys.
/// Those happen when the shell next starts and reads the config it wrote.
fn handle_preset_offline(arguments: &[String]) -> Result<Option<String>, String> {
    let folder = bw_core::paths::presets_dir();
    let name = || -> Result<&String, String> {
        arguments
            .get(1)
            .ok_or_else(|| "that needs a preset name".to_owned())
    };

    match arguments.first().map(String::as_str) {
        Some("list") => {
            let listed: Vec<String> = bw_core::preset::list(&folder)
                .into_iter()
                // A preset that will not parse is named with its reason rather
                // than left out: the file is still there, and only saying so
                // tells anybody that.
                .map(|summary| match summary.problem {
                    Some(problem) => format!("{} — {problem}", summary.name),
                    None => summary.name,
                })
                .collect();
            // Nothing at all rather than a blank line.
            Ok((!listed.is_empty()).then(|| listed.join("\n")))
        }
        Some("save") => {
            let path = bw_core::paths::config_file();
            let config = bw_core::config::load(&path).map_err(|error| error.to_string())?;
            let description = arguments.get(2..).unwrap_or_default().join(" ");
            bw_core::preset::save(
                &folder,
                name()?,
                &description,
                &serde_json::to_value(&config).expect("config is serialisable"),
                true,
            )
            .map_err(|error| error.to_string())?;
            Ok(None)
        }
        Some("apply") => {
            let path = bw_core::paths::config_file();
            let config = bw_core::config::load(&path).map_err(|error| error.to_string())?;
            let stored =
                bw_core::preset::load(&folder, name()?).map_err(|error| error.to_string())?;

            let mut json = serde_json::to_value(&config).expect("config is serialisable");
            let paths: Vec<String> = bw_core::preset::compare(&json, &stored.config)
                .changes
                .into_iter()
                .map(|change| change.path)
                .collect();
            bw_core::preset::apply(&mut json, &stored.config, &paths)
                .map_err(|error| error.to_string())?;

            let updated: bw_core::Config =
                serde_json::from_value(json).map_err(|error| error.to_string())?;
            bw_core::config::save(&path, &updated).map_err(|error| error.to_string())?;
            Ok(None)
        }
        Some("remove") => {
            bw_core::preset::remove(&folder, name()?).map_err(|error| error.to_string())?;
            Ok(None)
        }
        _ => Err(USAGE.to_owned()),
    }
}
