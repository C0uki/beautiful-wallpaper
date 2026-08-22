// The shell's windows are its UI; a console window behind them is not.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! Entry point.
//!
//! `bw.exe` with no arguments starts the shell. With arguments it is a client:
//! `bw.exe wallpapers apply <path>` talks to the running instance, the same way
//! `qs ipc call wallpapers apply <path>` does upstream.

use std::time::Duration;

use bw_shell::commands::{self, event};
use bw_shell::providers::Resources;
use bw_shell::services;
use bw_shell::state::AppState;
use bw_shell::{cli, surfaces};
use tauri::{Emitter, Manager};

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_env("BW_LOG")
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    // Treat any argument as a CLI request rather than starting a second shell.
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    if !arguments.is_empty() {
        std::process::exit(cli::run(&arguments));
    }

    let state = match AppState::load() {
        Ok(state) => state,
        Err(error) => {
            eprintln!("beautiful-wallpaper could not read its config: {error}");
            std::process::exit(1);
        }
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_single_instance::init(
            |app, arguments, _cwd| {
                // A second launch is a CLI call: hand its arguments to the running
                // shell rather than opening a duplicate set of surfaces.
                if let Err(error) = cli::dispatch(app, &arguments[1..]) {
                    tracing::warn!(%error, "could not handle a second-instance request");
                }
            },
        ))
        .manage(state.clone())
        .invoke_handler(tauri::generate_handler![
            commands::get_config,
            commands::set_config_value,
            commands::get_theme,
            commands::set_mode,
            commands::get_states,
            commands::toggle_state,
            commands::set_state,
            commands::list_wallpapers,
            commands::apply_wallpaper,
            commands::random_wallpaper,
            commands::thumbnail_for,
            commands::search_online_wallpapers,
            commands::download_wallpaper,
            commands::set_api_key,
            commands::media_command,
        ])
        .setup(move |app| {
            let handle = app.handle().clone();

            for surface in surfaces::ALL {
                if let Err(error) = surfaces::ensure(&handle, surface) {
                    tracing::error!(%error, surface = surface.label, "could not create a surface");
                }
            }

            // Generate the first palette before anything paints, so no surface
            // renders against fallback colours.
            match services::theme::regenerate(&state) {
                Ok(theme) => {
                    let _ = handle.emit(event::THEME_CHANGED, &theme);
                }
                Err(error) => tracing::error!(%error, "could not generate the initial theme"),
            }

            // Kept alive for the life of the app; dropping it stops the watch.
            let watcher = services::config::watch(handle.clone(), state.clone());
            app.manage(WatcherHandle(watcher));

            spawn_providers(handle, state.clone());
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("the shell failed to start");
}

/// Holds the config watcher so it is not dropped at the end of `setup`.
struct WatcherHandle(#[allow(dead_code)] Option<notify::RecommendedWatcher>);

/// Starts the timers that push system readings to the surfaces.
fn spawn_providers(app: tauri::AppHandle, state: AppState) {
    let resource_interval = Duration::from_millis(state.config().resources.poll_interval.into());

    // Resources and media are sampled on a plain thread: `sysinfo` and the SMTC
    // calls both block, and neither belongs on the UI thread.
    {
        let app = app.clone();
        std::thread::spawn(move || {
            let mut resources = Resources::new();
            loop {
                let _ = app.emit(event::RESOURCES, resources.sample());
                let _ = app.emit(event::BATTERY, bw_shell::providers::battery());
                std::thread::sleep(resource_interval);
            }
        });
    }

    {
        let app = app.clone();
        std::thread::spawn(move || loop {
            let _ = app.emit(event::MEDIA, bw_shell::providers::media());
            std::thread::sleep(Duration::from_secs(1));
        });
    }

    // Workspaces need an async client, and only matter when a tiling window
    // manager is running.
    {
        let port = state.config().windows.glazewm.port;
        tauri::async_runtime::spawn(async move {
            loop {
                let state = bw_shell::providers::workspaces(port).await;
                let _ = app.emit(event::WORKSPACES, &state);
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        });
    }
}
