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
use bw_shell::state::{
    AppState, BrightnessHandle, CaptureHandle, CatalogueHandle, ChatBusy, ChatStore,
    DesktopMenuHandle, DockHandle, IdleHandle, KeyReport, MicHandle, MixerHandle,
    NotificationStore, PersistentStore, PresetUndo, ShelfStore, TodoStore, VolumeHandle,
};
use bw_shell::surfaces::later;
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
    //
    // The ones that mean something with nothing running are answered outright.
    // The rest fall through into Tauri, because the single-instance plugin is
    // what reaches the running shell: exiting before the builder, as this used
    // to, left `dispatch` with no caller and made every such request report
    // "not running" whether a shell was up or not.
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    if let Some(code) = cli::run(&arguments) {
        std::process::exit(code);
    }

    // Windows are built on this thread, and building one needs it to be in a
    // single-threaded apartment — Tao asks for that with `OleInitialize`, which
    // fails outright on a thread already in a multi-threaded one. Whoever calls
    // COM first decides, and several of the watchers ask for a multi-threaded
    // apartment on whatever thread builds them, so leaving it to chance means
    // the shell starts or panics depending on the order things run in. Say it
    // here instead: a later `CoInitializeEx` for the other kind fails harmlessly
    // and every one of those calls already ignores its result.
    #[cfg(windows)]
    unsafe {
        use windows::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
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
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
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
        .manage(commands::VisualizerWatch::default())
        .invoke_handler(tauri::generate_handler![
            commands::get_config,
            commands::set_config_value,
            commands::get_theme,
            commands::set_mode,
            commands::get_states,
            commands::toggle_state,
            commands::set_state,
            commands::click_tray_icon,
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
            commands::latest_readings,
            commands::post_notification,
            commands::dismiss_notification,
            commands::clear_notifications,
            commands::get_volume,
            commands::set_volume,
            commands::set_muted,
            commands::step_volume,
            commands::get_brightness,
            commands::set_brightness,
            commands::step_brightness,
            commands::set_night_light,
            commands::get_mic,
            commands::set_mic,
            commands::set_mic_muted,
            commands::get_audio_sessions,
            commands::set_session_volume,
            commands::set_session_muted,
            commands::get_radios,
            commands::set_radio,
            commands::scan_wifi,
            commands::connect_wifi,
            commands::disconnect_wifi,
            commands::get_bluetooth_devices,
            commands::scan_bluetooth,
            commands::pair_bluetooth,
            commands::answer_bluetooth_pairing,
            commands::unpair_bluetooth,
            commands::connect_bluetooth,
            commands::get_idle_inhibit,
            commands::set_idle_inhibit,
            commands::watch_visualizer,
            commands::get_power_mode,
            commands::set_power_mode,
            commands::get_system_info,
            commands::get_todos,
            commands::add_todo,
            commands::set_todo_done,
            commands::remove_todo,
            commands::clear_done_todos,
            commands::reorder_todo,
            commands::get_persistent,
            commands::set_persistent_value,
            commands::activate_window,
            commands::launch_app,
            commands::get_lyrics,
            commands::set_pinned,
            commands::has_ai_key,
            commands::set_ai_key,
            commands::list_ai_models,
            commands::translate,
            commands::get_chat,
            commands::send_chat,
            commands::clear_chat,
            commands::retry_chat,
            commands::pick_files,
            commands::search_booru,
            commands::get_launcher_results,
            commands::launch_entry,
            commands::run_command,
            commands::start_capture,
            commands::finish_capture,
            commands::cancel_capture,
            commands::can_read_text,
            commands::get_session_actions,
            commands::run_session_action,
            commands::get_desktop_menu_items,
            commands::place_desktop_menu,
            commands::run_desktop_menu_item,
            commands::toggle_desktop_menu,
            commands::get_shelf_items,
            commands::add_to_shelf,
            commands::remove_from_shelf,
            commands::clear_shelf,
            commands::open_shelf_item,
            commands::reveal_shelf_item,
            commands::drag_from_shelf,
            commands::get_screen_chrome,
            commands::get_overlay_layout,
            commands::get_crosshair,
            commands::toggle_overlay_widget,
            commands::get_presets,
            commands::save_preset,
            commands::remove_preset,
            commands::compare_preset,
            commands::apply_preset,
            commands::has_preset_undo,
            commands::undo_preset,
            commands::get_key_report,
            commands::retry_keys,
            commands::detect_window_manager,
        ])
        .setup(move |app| {
            // Arguments still here means the single-instance plugin found no
            // shell to hand them to and let this process become the primary
            // one: its secondary path sends the request on and exits before
            // ever reaching here. Opening the whole desktop because someone
            // asked to toggle a sidebar would be surprising, so stop instead.
            if !arguments.is_empty() {
                eprintln!("beautiful-wallpaper is not running");
                std::process::exit(1);
            }

            let handle = app.handle().clone();

            // Generate the first palette before anything paints, so no surface
            // renders against fallback colours. Ahead of the surfaces now, so
            // the first one to exist already has one to read.
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
            app.manage(start_brightness_watch(&handle));
            app.manage(start_mic_watch(&handle));
            app.manage(start_mixer(&handle));
            app.manage(IdleHandle::default());
            app.manage(TodoStore::default());
            app.manage(PersistentStore::default());
            app.manage(start_dock_watch(&handle));
            app.manage(ChatStore::default());
            app.manage(ChatBusy::default());
            app.manage(start_app_scan(&handle));
            app.manage(CaptureHandle::default());
            app.manage(DesktopMenuHandle::default());
            app.manage(ShelfStore::default());
            app.manage(PresetUndo::default());
            app.manage(KeyReport::default());
            app.manage(services::integration::Integration::default());
            app.manage(services::listener::ListenerHandle::default());
            // After the handle is managed: hiding the taskbar is held by a
            // guard that lives in it.
            services::integration::apply(&handle);
            services::listener::apply(&handle);

            spawn_readings(&handle, &state);

            // The bar, the dock, the readout and the banners, drawn natively:
            // up in a second, beside everything else rather than behind it.
            {
                let reserving = handle.clone();
                let clicking = handle.clone();
                let config = state.config();
                bw_native::bar_shown(config.bar.enable);
                bw_native::bars(surfaces::bar_layout(&config));
                commands::emit_bar_settings(&config);
                bw_native::spawn(
                    Some(bw_native::BarHooks {
                        place: Box::new(move |hwnd, monitor, vertical, thickness| {
                            surfaces::adopt_native_bar(
                                &reserving, hwnd, monitor, vertical, thickness,
                            )
                        }),
                    }),
                    Box::new(|| {
                        commands::latest_readings()
                            .into_iter()
                            .map(|(event, value)| (event.to_owned(), value))
                            .collect()
                    }),
                    Box::new(move |action| native_pressed(&clicking, action)),
                );
                commands::emit_dock_settings(&handle);
                // The window watcher only sends the icons when a window
                // changes.
                commands::rebuild_dock(&handle);
            }

            // One surface to a turn of the event loop, rather than all of them
            // back to back. See the note on `later`.
            let make = |surface: &'static surfaces::Surface| {
                let each = handle.clone();
                Box::new(move || {
                    if let Err(error) = surfaces::ensure(&each, surface) {
                        tracing::error!(%error, surface = surface.label, "could not create a surface");
                    }
                }) as Box<dyn FnOnce() + Send>
            };

            // What is on screen first, then the rest of the startup, which
            // reaches for those; the surfaces only ever opened come after, so
            // a slow machine shows a desktop in seconds rather than once all
            // of them exist.
            let mut steps: Vec<Box<dyn FnOnce() + Send>> =
                surfaces::FIRST.iter().map(make).collect();
            // The frame, only when there is one to draw.
            let framing = handle.clone();
            steps.push(Box::new(move || services::chrome::emit(&framing)));
            let rest = handle.clone();
            let state = state.clone();
            steps.push(Box::new(move || finish_starting(rest, state)));
            steps.extend(surfaces::AFTER.iter().map(make));
            // The passive half is masked by transparency rather than by a
            // region, and its window has to exist before that can be set.
            let overlays = handle.clone();
            steps.push(Box::new(move || {
                services::overlay::make_passive_clickthrough(&overlays);
                services::overlay::apply(&overlays);
            }));
            let first = handle.clone();
            std::thread::spawn(move || {
                wait_for_profile(&first);
                in_turn(first, steps.into());
            });

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("the shell failed to start")
        .run(|app, event| {
            // Tauri's default exit path calls `process::exit`, which unwinds
            // nothing — so anything the shell changed about Windows itself
            // would stay changed, with no shell left to change it back. Both
            // of these are held by guards whose `Drop` would otherwise never
            // run: a hidden taskbar, and the edge of the screen the bar
            // reserved.
            if matches!(event, tauri::RunEvent::Exit) {
                services::integration::restore(app);
                surfaces::release_reservations(app);
                // And out, rather than back into the event loop. At sign-out,
                // restart or shutdown, tao ends its loop from inside
                // `WM_ENDSESSION` but goes on pumping messages until Windows
                // gets round to killing the process, and the next window
                // event panics with "cannot move state from Destroyed".
                // Tauri's own exit finishes the same way: this cleanup, then
                // `process::exit`.
                app.cleanup_before_exit();
                std::process::exit(0);
            }
        });
}

/// Holds the config watcher so it is not dropped at the end of `setup`.
struct WatcherHandle(#[allow(dead_code)] Option<notify::RecommendedWatcher>);

/// Waits, for up to three minutes, until no WebView2 from an earlier run still has
/// the shell's profile open.
///
/// WebView2 hands a new webview to whatever browser process already has its
/// profile folder open, and a quit — an update's restart above all — leaves
/// that process shutting down for half a minute after the shell itself has
/// gone. Handed to it, the first surface was never finished, and the shell
/// sat running with no bar, no dock and nothing else. The browser keeps the
/// profile's `lockfile` open unshared until it exits, so being able to open it
/// that way means there is nothing left to join.
///
/// Three minutes because a whole one was measured going by, after the shell
/// was killed, with the old browser still holding on.
fn wait_for_profile(handle: &tauri::AppHandle) {
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;

        const SHARING_VIOLATION: i32 = 32;
        let Ok(folder) = handle.path().app_local_data_dir() else {
            return;
        };
        let lock = folder.join("EBWebView").join("lockfile");
        let started = std::time::Instant::now();
        while started.elapsed() < Duration::from_secs(180) {
            match std::fs::OpenOptions::new()
                .read(true)
                .share_mode(0)
                .open(&lock)
            {
                Err(error) if error.raw_os_error() == Some(SHARING_VIOLATION) => {
                    std::thread::sleep(Duration::from_millis(250));
                }
                // Opened, or not there at all: nobody holds it.
                _ => break,
            }
        }
        if started.elapsed() >= Duration::from_millis(250) {
            tracing::info!(
                waited_ms = started.elapsed().as_millis() as u64,
                "waited for the last run's WebView2 to exit"
            );
        }
    }
    #[cfg(not(windows))]
    let _ = handle;
}

/// Runs `steps` one to a turn, each queued only once the one before has run.
///
/// Queued all at once, they stood ahead of everything sent to the surfaces in
/// the meantime: an event reaches a page by a turn of this same loop, so every
/// reading, the weather and the active window's title waited behind all
/// nineteen surfaces — the bar sat half empty for thirty seconds on a slow
/// start, though each of them was ready within one.
fn in_turn(
    handle: tauri::AppHandle,
    mut steps: std::collections::VecDeque<Box<dyn FnOnce() + Send>>,
) {
    let Some(step) = steps.pop_front() else {
        return;
    };
    let next = handle.clone();
    later(&handle, move || {
        step();
        in_turn(next, steps);
    });
}

/// The rest of the startup, once every surface has a window.
///
/// Each of these reaches for a particular surface and does nothing at all if
/// it is not there yet, with nothing to try again later — so this runs behind
/// the whole queue rather than alongside it.
fn finish_starting(handle: tauri::AppHandle, state: AppState) {
    // One to a turn, like the surfaces ahead of them, and for the same reason:
    // run end to end these took long enough on their own to leave the windows
    // silent past the five seconds Windows waits before it ghosts one. They
    // keep their order — each still runs behind the surfaces it reaches for,
    // and behind each other — but the loop gets to breathe between them.

    // So the first click has somewhere to go.
    let each = handle.clone();
    later(&handle, move || services::deskmenu::apply(&each));

    // So what the wallpaper surface draws can be pressed: Explorer's desktop
    // takes every click over it.
    #[cfg(windows)]
    {
        let each = handle.clone();
        bw_shell::platform::deskclick::forward_pointer(move || {
            let window = each.get_webview_window(&surfaces::background_label())?;
            Some(window.hwnd().ok()?.0 as isize)
        });
    }

    // So a key pressed the instant the shell is up has something to open. And
    // before the first-run screen, which shows what Windows refused.
    let each = handle.clone();
    later(&handle, move || services::hotkeys::apply(&each));

    later(&handle.clone(), move || {
        // A machine that has not been through the first run opens it itself.
        // There is nothing to see past it on a fresh install: no wallpaper has
        // been chosen, so the palette has no source, and no key has been proven
        // to work.
        if !handle.state::<PersistentStore>().0.get().first_run.done {
            if let Some(states) = state.set_state("wizardOpen", true) {
                surfaces::apply_states(&handle, &states);
                let _ = handle.emit(event::STATE_CHANGED, &states);
            }
        }

        services::updater::start(&handle);
        spawn_providers(handle, state);
        tracing::info!("the shell is up");
    });
}

/// Starts the readings that only ever send: the bar's resources, network and
/// battery, and the weather.
///
/// Ahead of the surfaces rather than behind them, as everything in
/// [`spawn_providers`] is: none of these reaches for a surface, and behind
/// nineteen of them on a slow start the bar went thirty seconds with nothing
/// to show. A surface still loading misses a sample, and has the next.
fn spawn_readings(app: &tauri::AppHandle, state: &AppState) {
    services::weather::start(app);

    let resource_interval = Duration::from_millis(state.config().resources.poll_interval.into());
    // Sampled on a plain thread: `sysinfo` blocks, and does not belong on the
    // UI thread.
    let app = app.clone();
    std::thread::spawn(move || {
        let mut resources = Resources::new();
        let mut network = Network::new();
        loop {
            commands::emit_reading(&app, event::RESOURCES, resources.sample());
            commands::emit_reading(&app, event::NETWORK, network.sample());
            commands::emit_reading(&app, event::BATTERY, bw_shell::providers::battery());
            std::thread::sleep(resource_interval);
        }
    });
}

fn spawn_providers(app: tauri::AppHandle, state: AppState) {
    // Media is sampled on a plain thread: the SMTC calls block, and do not
    // belong on the UI thread.
    {
        let app = app.clone();
        std::thread::spawn(move || loop {
            let mut media = bw_shell::providers::media();
            media.source = bw_shell::providers::app_name(&media.source, || {
                app.state::<CatalogueHandle>().items()
            });
            commands::emit_reading(&app, event::MEDIA, media);
            // The title bar changes as fast as the user alt-tabs, so this is
            // sampled at the same rate as the transport state.
            let active = bw_shell::providers::active_window();
            commands::emit_reading(&app, event::ACTIVE_WINDOW, active);

            // Rides this timer rather than one of its own: it reads the same
            // kind of window state, and a desktop that has stopped taking
            // clicks can wait a second to be named.
            #[cfg(windows)]
            surfaces::warn_about_surfaces_that_swallow_a_monitor(&app);

            // Explorer takes the wallpaper surface down with it the moment it
            // goes, and only announces its return once its taskbar is ready —
            // forty seconds later, on the machine. Waiting for that meant a
            // WebView2 error in the log for every call made on the dead
            // surface in between; looking every second keeps it to a second's.
            surfaces::forget_dead_background(&app);
            surfaces::restore_owed_background(&app);

            std::thread::sleep(Duration::from_secs(1));
        });
    }

    // The notification area is hosted by the shell, which hears of every
    // change as it happens. Looked at again every few seconds as well, so an
    // application that exited without taking its icon away is noticed then.
    {
        #[cfg(windows)]
        {
            let app = app.clone();
            bw_shell::platform::tray::host(move || {
                commands::emit_reading(&app, event::TRAY, bw_shell::providers::tray_icons());
            });
        }
        let app = app.clone();
        std::thread::spawn(move || loop {
            commands::emit_reading(&app, event::TRAY, bw_shell::providers::tray_icons());
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
                // Most machines have no GlazeWM at all, and asked every two
                // seconds anyway, each time with a new connection. Nobody
                // there, it is asked again in half a minute.
                let away = state.source.is_none();
                commands::emit_reading(&app, event::WORKSPACES, &state);
                tokio::time::sleep(Duration::from_secs(if away { 30 } else { 2 })).await;
            }
        });
    }
}

/// Does what was pressed on a native surface, as the webview's buttons do.
fn native_pressed(app: &AppHandle, action: bw_native::Action) {
    match action {
        bw_native::Action::Open(what) => {
            let arguments: Vec<String> = match what.as_str() {
                "randomWallpaper" => vec!["wallpapers".into(), "random".into()],
                _ => vec![what, "toggle".into()],
            };
            // On the main thread, as a request from a second launch is.
            let each = app.clone();
            let _ = app.run_on_main_thread(move || {
                if let Err(error) = cli::dispatch(&each, &arguments) {
                    tracing::warn!(%error, "the native bar's button did nothing");
                }
            });
        }
        // The media calls block, and the bar's thread draws it.
        bw_native::Action::Media(what) => {
            std::thread::spawn(move || {
                if let Err(error) = commands::media_command(what) {
                    tracing::warn!(%error, "the native bar could not control the media");
                }
            });
        }
        bw_native::Action::Tray(icon, secondary) => commands::click_tray_icon(
            icon["window"].as_str().unwrap_or_default().to_owned(),
            icon["id"].as_u64().unwrap_or_default() as u32,
            icon["callbackMessage"].as_u64().unwrap_or_default() as u32,
            secondary,
        ),
        bw_native::Action::Dismiss(id) => {
            if app.state::<NotificationStore>().0.dismiss(id) {
                commands::emit_notifications(app);
            }
        }
        // As the webview dock's `activate`: start it when nothing of it is
        // open, otherwise bring up the window after the one in front, so a
        // program with several windows cycles through them.
        bw_native::Action::DockActivate(dock_app) => {
            let windows = dock_app["windows"].as_array().cloned().unwrap_or_default();
            if windows.is_empty() {
                let path = dock_app["executable"]
                    .as_str()
                    .unwrap_or_default()
                    .to_owned();
                if let Err(error) = commands::launch_app(app.clone(), path) {
                    tracing::warn!(%error, "the dock could not start an application");
                }
                return;
            }
            let current = windows
                .iter()
                .position(|window| window["active"].as_bool().unwrap_or(false));
            let next = &windows[current.map_or(0, |at| (at + 1) % windows.len())];
            let id = next["id"].as_str().unwrap_or_default().to_owned();
            let active = dock_app["active"].as_bool().unwrap_or(false);
            // ponytail: a refused activation only flashes the window; the
            // webview dock also bounced its icon.
            if commands::activate_window(id, active) == commands::ActivateOutcome::Gone {
                commands::rebuild_dock(app);
            }
        }
        bw_native::Action::DockPin(path, pinned) => {
            if let Err(error) = commands::set_pinned(app.state(), path, pinned) {
                tracing::warn!(%error, "the dock could not pin an application");
            }
            commands::rebuild_dock(app);
        }
        bw_native::Action::DockMove(moved, target) => {
            let state = app.state::<AppState>();
            let pinned = state.config().dock.pinned_apps;
            if let Some(next) = bw_core::dock::reorder(&pinned, &moved, &target) {
                if let Err(error) =
                    state.set_config_value("dock.pinnedApps", serde_json::json!(next))
                {
                    tracing::warn!(%error, "the dock could not move an icon");
                }
                commands::rebuild_dock(app);
            }
        }
        bw_native::Action::DockDrop(dock_app, files) => {
            // Only ever a program that is really there, whatever the dock
            // says, and off this thread: starting one can take a moment.
            let program = dock_app["executable"]
                .as_str()
                .unwrap_or_default()
                .to_owned();
            let path = std::path::Path::new(&program);
            let runnable = path.is_file()
                && path
                    .extension()
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("exe"));
            if !runnable {
                tracing::warn!(%program, "files were dropped on something that is not a program");
                return;
            }
            #[cfg(windows)]
            std::thread::spawn(move || {
                if let Err(error) = bw_shell::platform::launch::open_with(&program, &files) {
                    tracing::warn!(%error, "could not open the files dropped on the dock");
                }
            });
            #[cfg(not(windows))]
            let _ = files;
        }
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

        let watcher = bw_shell::platform::audio::VolumeWatcher::for_output(move |reading| {
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

/// Watches the microphone.
///
/// No readout: the OSD is for things the user changed with a key they just
/// pressed, and nothing on a standard keyboard changes input gain.
fn start_mic_watch(app: &AppHandle) -> MicHandle {
    #[cfg(windows)]
    {
        let handle = app.clone();
        let watcher = bw_shell::platform::audio::VolumeWatcher::for_input(move |reading| {
            let reading: bw_shell::providers::VolumeReading = reading.into();
            let _ = handle.emit(event::MIC, reading);
        });

        match watcher {
            Ok(watcher) => MicHandle(VolumeHandle::new(Some(watcher))),
            Err(error) => {
                // A machine with no microphone is entirely ordinary.
                tracing::debug!(%error, "no microphone to watch");
                MicHandle(VolumeHandle::new(None))
            }
        }
    }
    #[cfg(not(windows))]
    {
        let _ = app;
        MicHandle(VolumeHandle::new())
    }
}

/// Opens the per-application mixer.
///
/// The callback only says "something changed"; the sidebar re-reads the list.
/// Sessions come and go constantly and the list is short, so re-reading is
/// both cheaper to get right and safer than tracking a diff of COM objects
/// that can disappear between one call and the next.
fn start_mixer(app: &AppHandle) -> MixerHandle {
    #[cfg(windows)]
    {
        let handle = app.clone();
        let mixer = bw_shell::platform::mixer::Mixer::new(move || {
            // `try_state`, not `state`: a session can change between the mixer
            // being constructed and the handle being managed, and `state`
            // panics on a type nobody has registered yet.
            let Some(handles) = handle.try_state::<MixerHandle>() else {
                return;
            };
            let _ = handle.emit(event::AUDIO_SESSIONS, handles.list());
        });

        match mixer {
            Ok(mixer) => MixerHandle::new(Some(mixer)),
            Err(error) => {
                tracing::warn!(%error, "could not open the per-application mixer");
                MixerHandle::new(None)
            }
        }
    }
    #[cfg(not(windows))]
    {
        let _ = app;
        MixerHandle::new()
    }
}

/// Starts the brightness worker, and shows the readout on every change.
///
/// Mirrors the volume watcher, including swallowing the first reading: that
/// one is the current level rather than a change, and showing it would flash
/// the readout on every launch.
fn start_brightness_watch(app: &AppHandle) -> BrightnessHandle {
    #[cfg(windows)]
    {
        use std::sync::atomic::{AtomicBool, Ordering};

        let handle = app.clone();
        let seen_first = AtomicBool::new(false);

        let control = bw_shell::platform::brightness::BrightnessControl::new(move |percent| {
            let _ = handle.emit(event::BRIGHTNESS, percent);

            if seen_first.swap(true, Ordering::Relaxed) {
                show_osd(&handle, "brightness", f32::from(percent), false);
            }
        });

        // The night light is a config setting, so it has to be re-applied at
        // startup — the gamma ramp is cleared when the shell exits.
        let config = app.state::<AppState>().config();
        if config.sidebar.night_light.enable {
            control.set_night_light(Some(config.sidebar.night_light.temperature));
        }

        BrightnessHandle::new(Some(control))
    }
    #[cfg(not(windows))]
    {
        let _ = app;
        BrightnessHandle::new()
    }
}

/// Watches for windows opening and closing so the dock stays current.
///
/// Deliberately event-driven rather than polled: an icon that lingers for a
/// second after its application closes is what makes a dock feel broken.
fn start_dock_watch(app: &AppHandle) -> DockHandle {
    #[cfg(windows)]
    {
        // A title changes many times a second in a browser or a player, and
        // every change rebuilt the whole list — each window's process opened,
        // its icon looked up — and sent it out. Changes now only ask for a
        // rebuild; this thread does one for each burst, a fifth of a second
        // after it starts, and the list goes out only when it differs.
        let (rebuild, asked) = std::sync::mpsc::channel::<()>();
        {
            let handle = app.clone();
            std::thread::Builder::new()
                .name("bw-dock".to_owned())
                .spawn(move || {
                    while asked.recv().is_ok() {
                        std::thread::sleep(Duration::from_millis(200));
                        while asked.try_recv().is_ok() {}
                        let (Some(dock), Some(state)) = (
                            handle.try_state::<DockHandle>(),
                            handle.try_state::<AppState>(),
                        ) else {
                            continue;
                        };
                        commands::emit_reading(&handle, event::DOCK, dock.items(&state.config()));
                    }
                })
                .expect("could not start the thread that rebuilds the dock");
        }

        let handle = app.clone();
        let watcher = bw_shell::platform::windows::WindowWatcher::new(move |change| {
            use bw_shell::platform::windows::Change;

            // `try_state`, not `state`: the first event can arrive between the
            // watcher being built and the handle being managed, and `state`
            // panics on a type nobody has registered yet.
            let (Some(_), Some(state)) = (
                handle.try_state::<DockHandle>(),
                handle.try_state::<AppState>(),
            ) else {
                return;
            };

            // The user went back to another program, or clicked somewhere that
            // is not the shell, which puts a sidebar away, as the original's
            // focus grab does. Neither had any other way out but Escape, and
            // Escape only reaches a sidebar holding the keyboard. The click
            // matters when the sidebar never took the foreground: clicking the
            // program that kept it moves nothing, so there is no foreground
            // change to see. A click on the desktop counts too: the wallpaper
            // surface lives inside Explorer's window, so it is not the shell's.
            let elsewhere = matches!(
                change,
                Change::Foreground | Change::Click { on_shell: false }
            );
            let open = state.states();
            if elsewhere && (open.sidebar_left_open || open.sidebar_right_open) {
                state.set_state("sidebarLeftOpen", false);
                if let Some(states) = state.set_state("sidebarRightOpen", false) {
                    surfaces::apply_states(&handle, &states);
                    let _ = handle.emit(event::STATE_CHANGED, &states);
                }
            }

            if matches!(change, Change::Click { .. }) {
                return;
            }
            let _ = rebuild.send(());
        });

        DockHandle::new(Some(watcher))
    }
    #[cfg(not(windows))]
    {
        let _ = app;
        DockHandle::new()
    }
}

/// Scans the installed applications, and pushes the list when it changes.
///
/// The scan opens a COM object per Start-menu shortcut and rasterises an icon
/// for each, which is far too slow to do while the user waits with the search
/// box open — so it runs once in the background and again when the Start menu
/// changes underneath it.
fn start_app_scan(app: &AppHandle) -> CatalogueHandle {
    #[cfg(windows)]
    {
        let handle = app.clone();
        let catalogue = bw_shell::platform::apps::Catalogue::new(move || {
            // The payload would be the whole list; the overview re-runs its
            // own query instead, which is what it would have to do anyway.
            let _ = handle.emit(event::APPS, ());
        });
        CatalogueHandle::new(Some(catalogue))
    }
    #[cfg(not(windows))]
    {
        let _ = app;
        CatalogueHandle::new()
    }
}

/// Shows the readout with a value. `bw-native` times it out, and a burst of
/// changes — holding a volume key — keeps it up rather than letting an early
/// timer close it mid-press.
#[cfg(windows)]
fn show_osd(app: &AppHandle, kind: &str, value: f32, muted: bool) {
    let state = app.state::<AppState>();
    let config = state.config();
    if !config.osd.enable {
        return;
    }

    let timeout = Duration::from_millis(u64::from(config.osd.timeout));
    bw_native::osd(kind, value, muted, timeout, config.osd.position == "bottom");
}
