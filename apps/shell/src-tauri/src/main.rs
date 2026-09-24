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
    AppState, BrightnessHandle, CaptureHandle, CatalogueHandle, ChatBusy, ChatStore, ChromeState,
    DesktopMenuHandle, DockHandle, IdleHandle, KeyReport, MicHandle, MixerHandle,
    NotificationStore, PersistentStore, PresetUndo, ShelfStore, TodoStore, VolumeHandle,
};
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
            commands::set_surface_revealed,
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
            commands::get_idle_inhibit,
            commands::set_idle_inhibit,
            commands::get_system_info,
            commands::get_todos,
            commands::add_todo,
            commands::set_todo_done,
            commands::remove_todo,
            commands::clear_done_todos,
            commands::reorder_todo,
            commands::get_persistent,
            commands::set_persistent_value,
            commands::get_dock_items,
            commands::activate_window,
            commands::launch_app,
            commands::set_pinned,
            commands::has_ai_key,
            commands::set_ai_key,
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
            commands::get_hot_corners,
            commands::run_hot_corner,
            commands::scroll_hot_corner,
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
            app.manage(ChromeState::default());
            app.manage(PresetUndo::default());
            app.manage(KeyReport::default());
            app.manage(services::integration::Integration::default());
            app.manage(services::listener::ListenerHandle::default());
            // After the handle is managed: hiding the taskbar is held by a
            // guard that lives in it.
            services::integration::apply(&handle);
            services::listener::apply(&handle);

            // One surface to a turn of the event loop, rather than nineteen
            // back to back. See the note on `later`.
            for surface in surfaces::ALL {
                let each = handle.clone();
                later(&handle, move || {
                    if let Err(error) = surfaces::ensure(&each, surface) {
                        tracing::error!(%error, surface = surface.label, "could not create a surface");
                    }
                });
            }

            // Behind every one of them in the queue, because each step of it
            // needs a surface to have a window before it can touch it.
            let rest = handle.clone();
            let state = state.clone();
            later(&handle, move || finish_starting(rest, state));

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
            }
        });
}

/// Holds the config watcher so it is not dropped at the end of `setup`.
struct WatcherHandle(#[allow(dead_code)] Option<notify::RecommendedWatcher>);

/// Starts the timers that push system readings to the surfaces.
/// Queues `work` for a later turn of the event loop.
///
/// Creating a webview takes long enough that nineteen of them back to back
/// leave the main thread without a message pump for well past the five seconds
/// Windows waits before it decides a window is hung. What it puts up then is a
/// full-screen "not responding" ghost over `screenChrome` — transparent but
/// not layered, which is exactly the pair that swallows every click on the
/// desktop rather than passing it through. Spread over the loop, the thread is
/// never silent long enough to be asked.
fn later(handle: &tauri::AppHandle, work: impl FnOnce() + Send + 'static) {
    if let Err(error) = handle.run_on_main_thread(work) {
        tracing::error!(%error, "could not queue a step of the startup");
    }
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

    // The hot corners need their window before they can be cut down to size.
    let each = handle.clone();
    later(&handle, move || services::chrome::apply(&each));

    // The passive half is masked by transparency rather than by a region, and
    // its window has to exist before that can be set.
    let each = handle.clone();
    later(&handle, move || {
        services::overlay::make_passive_clickthrough(&each);
        services::overlay::apply(&each);
    });

    // So the first click has somewhere to go.
    let each = handle.clone();
    later(&handle, move || services::deskmenu::apply(&each));

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

        spawn_providers(handle, state);
    });
}

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
            let active = bw_shell::providers::active_window();
            let fullscreen = active.fullscreen;
            let _ = app.emit(event::ACTIVE_WINDOW, active);

            // Whether anything is full-screen comes free with the foreground
            // window, and only matters when it changes: the decorations would
            // otherwise be told the same thing every second for hours.
            if let Some(chrome) = app.try_state::<ChromeState>() {
                if chrome.set_fullscreen(fullscreen) {
                    services::chrome::emit(&app);
                    services::chrome::apply(&app);
                }
            }

            // Rides this timer rather than one of its own: it reads the same
            // kind of window state, and a desktop that has stopped taking
            // clicks can wait a second to be named.
            #[cfg(windows)]
            surfaces::warn_about_surfaces_that_swallow_a_monitor(&app);

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
        let handle = app.clone();
        let watcher = bw_shell::platform::windows::WindowWatcher::new(move || {
            // `try_state`, not `state`: the first event can arrive between the
            // watcher being built and the handle being managed, and `state`
            // panics on a type nobody has registered yet.
            let (Some(dock), Some(state)) = (
                handle.try_state::<DockHandle>(),
                handle.try_state::<AppState>(),
            ) else {
                return;
            };
            let _ = handle.emit(event::DOCK, dock.items(&state.config()));
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
