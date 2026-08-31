//! Switching the desktop's right button on and off.
//!
//! The hook is a hack the user opts into, and `config.json` can be edited while
//! the shell is running, so this has to be as re-runnable as
//! [`hotkeys::apply`](super::hotkeys::apply): called at startup, and again
//! whenever `hacks` changes.

use tauri::{AppHandle, Manager};

use crate::state::{AppState, DesktopMenuHandle};

/// Registers or removes the hook to match the config.
///
/// Idempotent: with the setting unchanged this leaves the existing hook alone
/// rather than replacing it, so a config save that touched something else does
/// not cost a re-registration.
pub fn apply(app: &AppHandle) {
    let (Some(state), Some(handle)) = (
        app.try_state::<AppState>(),
        app.try_state::<DesktopMenuHandle>(),
    ) else {
        return;
    };

    let config = state.config();
    let wanted = config.hacks.desktop_menu && config.desktop_menu.enable;
    if wanted == handle.is_hooked() {
        return;
    }

    if !wanted {
        #[cfg(windows)]
        handle.set_hook(None);
        return;
    }

    #[cfg(windows)]
    {
        let app = app.clone();
        handle.set_hook(Some(crate::platform::deskclick::DesktopClickHook::new(
            move |(x, y)| {
                crate::commands::open_desktop_menu_at(&app, bw_core::menu::Placement { x, y });
            },
        )));
    }
}
