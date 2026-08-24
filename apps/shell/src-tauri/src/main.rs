// The shell's windows are its UI; a console window behind them is not.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! Entry point.
//!
//! `bw.exe` with no arguments starts the shell. With arguments it is a client:
//! `bw.exe wallpapers apply <path>` talks to the running instance, the same way
//! `qs ipc call wallpapers apply <path>` does upstream.

use std::time::Duration;

use bw_shell::commands::{self, event};
use bw_shell::providers::{Network, Resources};
use bw_shell::services;
use bw_shell::state::{AppState, NotificationStore, VolumeHandle};
use bw_shell::{cli, surfaces};
use tauri::{AppHandle, Emitter, Manager};

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
        .manage(surfaces::Reservations::default())
        .manage(NotificationStore::default())
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
            commands::get_monitors,
            commands::set_taskbar_visible,
            commands::get_notifications,
            commands::post_notification,
            commands::dismiss_notification,
            commands::clear_notifications,
            commands::get_volume,
            commands::set_volume,
            commands::set_muted,
            commands::step_volume,
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

            app.manage(start_volume_watch(&handle));

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
            let mut network = Network::new();
            loop {
                let _ = app.emit(event::RESOURCES, resources.sample());
                let _ = app.emit(event::NETWORK, network.sample());
                let _ = app.emit(event::BATTERY, bw_shell::providers::battery());
                std::thread::sleep(resource_interval);
            }
        });
    }

    {
        let app = app.clone();
        std::thread::spawn(move || loop {
            let _ = app.emit(event::MEDIA, bw_shell::providers::media());
            // The title bar changes as fast as the user alt-tabs, so this is
            // sampled at the same rate as the transport state.
            let _ = app.emit(event::ACTIVE_WINDOW, bw_shell::providers::active_window());
            std::thread::sleep(Duration::from_secs(1));
        });
    }

    // Reading the notification area means poking at Explorer across a process
    // boundary, which is far too expensive to do every second.
    {
        let app = app.clone();
        std::thread::spawn(move || loop {
            let _ = app.emit(event::TRAY, bw_shell::providers::tray_icons());
            std::thread::sleep(Duration::from_secs(5));
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

/// Starts watching the output volume, and shows the readout on every change.
///
/// The watcher pushes rather than polls, so the readout appears on the same
/// keypress that changed the volume. The very first reading is the current
/// level rather than a change, so it is recorded without showing anything —
/// otherwise the readout would flash on every launch.
fn start_volume_watch(app: &AppHandle) -> VolumeHandle {
    #[cfg(windows)]
    {
        use std::sync::atomic::{AtomicBool, Ordering};

        let handle = app.clone();
        let seen_first = AtomicBool::new(false);

        let watcher = bw_shell::platform::audio::VolumeWatcher::new(move |reading| {
            let reading: bw_shell::providers::VolumeReading = reading.into();
            let _ = handle.emit(event::VOLUME, reading);

            if seen_first.swap(true, Ordering::Relaxed) {
                show_osd(&handle, "volume", reading.percent, reading.muted);
            }
        });

        match watcher {
            Ok(watcher) => VolumeHandle::new(Some(watcher)),
            Err(error) => {
                // No output device, or an audio service that will not talk to
                // us. The shell runs; the readout simply has nothing to show.
                tracing::warn!(%error, "could not watch the output volume");
                VolumeHandle::new(None)
            }
        }
    }
    #[cfg(not(windows))]
    {
        let _ = app;
        VolumeHandle::new()
    }
}

/// Shows the readout with a value, and lets it time itself out.
///
/// The timeout lives here rather than in the surface so that a burst of
/// changes — holding a volume key — keeps the readout up instead of letting an
/// early timer close it mid-press.
#[cfg(windows)]
fn show_osd(app: &AppHandle, kind: &str, value: f32, muted: bool) {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::OnceLock;

    static GENERATION: OnceLock<AtomicU64> = OnceLock::new();
    let generation = GENERATION.get_or_init(|| AtomicU64::new(0));

    let state = app.state::<AppState>();
    let config = state.config();
    if !config.osd.enable {
        return;
    }

    let _ = app.emit(
        event::OSD,
        serde_json::json!({ "kind": kind, "value": value, "muted": muted }),
    );
    if let Err(error) = surfaces::set_visible(app, surfaces::OSD.label, true) {
        tracing::warn!(%error, "could not show the readout");
        return;
    }

    // Only the newest change gets to close the readout.
    let mine = generation.fetch_add(1, Ordering::Relaxed) + 1;
    let handle = app.clone();
    let timeout = Duration::from_millis(u64::from(config.osd.timeout));

    std::thread::spawn(move || {
        std::thread::sleep(timeout);
        if generation.load(Ordering::Relaxed) == mine {
            let _ = surfaces::set_visible(&handle, surfaces::OSD.label, false);
        }
    });
}
