//! Moving to a new release on its own (`windows.autoUpdate`).
//!
//! The releases page carries a `latest.json` next to the installers, signed
//! with a key whose public half is in `tauri.conf.json`; the plugin refuses an
//! installer that signature does not cover. What it then does on Windows is
//! start the installer and end this process on the spot with
//! `process::exit`, which runs no `Drop` and no `RunEvent::Exit`. So the shell
//! puts back what it changed about Windows first — the taskbar and the bar's
//! edge — exactly as it does on any other way out. The installer starts the
//! new version when it is done.

use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_updater::UpdaterExt;

use crate::commands::event;
use crate::state::{AppState, NotificationStore};

/// Checks a minute after starting, then every six hours, for as long as the
/// shell runs. The switch is read before each check, so turning it off needs
/// no restart.
pub fn start(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        // Not at startup itself: the surfaces are still being made, and an
        // update found then would tear them down half built.
        tokio::time::sleep(Duration::from_secs(60)).await;
        loop {
            if app.state::<AppState>().config().windows.auto_update {
                if let Err(error) = update(&app).await {
                    // Offline, GitHub down, a release still in draft: all
                    // ordinary, and all answered by the next check.
                    tracing::warn!(%error, "could not check for an update");
                }
            }
            tokio::time::sleep(Duration::from_secs(6 * 60 * 60)).await;
        }
    });
}

async fn update(app: &AppHandle) -> tauri_plugin_updater::Result<()> {
    let leaving = app.clone();
    let updater = app
        .updater_builder()
        .on_before_exit(move || {
            crate::services::integration::restore(&leaving);
            crate::surfaces::release_reservations(&leaving);
        })
        .build()?;

    let Some(update) = updater.check().await? else {
        return Ok(());
    };
    tracing::info!(version = %update.version, "installing an update");
    say(
        app,
        &format!(
            "Updating to {}. The shell restarts when it is done.",
            update.version
        ),
    );

    let installed = update.download_and_install(|_, _| {}, || {}).await;
    // Still here: the installer never started, after the taskbar and the
    // bar's edge were already given back for it. Take both again.
    crate::services::integration::apply(app);
    crate::surfaces::place_bars(app);
    installed
}

fn say(app: &AppHandle, text: &str) {
    let Some(store) = app.try_state::<NotificationStore>() else {
        return;
    };
    store
        .0
        .post(bw_core::NewNotification::from_shell("Update", text));
    let _ = app.emit(event::NOTIFICATIONS, store.0.list());
}
