//! Reading what other applications have posted to the Action Center.
//!
//! `UserNotificationListener` is the only supported way to do it, and it has
//! two gates in front of it. The process needs **package identity**, which
//! `platform::identity` is about; and the user has to **allow** it, in
//! Windows' own settings, which no code here can decide for them.
//!
//! The rules about what to do with what comes back — which of them are new,
//! how a toast's text becomes a summary and a body, what to call the sender —
//! are in `bw_core::listener` under tests. This is the part that needs
//! Windows.

use bw_core::listener;
use bw_core::NewNotification;
use windows::UI::Notifications::Management::{
    UserNotificationListener, UserNotificationListenerAccessStatus,
};
use windows::UI::Notifications::{KnownNotificationBindings, NotificationKinds};

/// How far the shell has got towards being able to read them.
///
/// Four states rather than a boolean, because the three ways this does not
/// work want three different sentences. "It is off" tells somebody nothing
/// about which of the two gates they are standing at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Access {
    /// No package identity, so the listener will not talk to this process at
    /// all. The sparse package is what fixes it — and a restart after it.
    NoIdentity,
    /// This Windows is too old for the listener, or it is otherwise absent.
    Unavailable,
    /// Identity is there and Windows has not been asked yet.
    Unasked,
    /// The user said no, in Windows' own settings. Only they can undo it.
    Denied,
    Allowed,
}

/// Asks Windows for access, and says where that got to.
///
/// Requesting is what puts the shell in the notification-access list; until
/// then there is nothing for the user to allow even if they want to.
pub fn request() -> Access {
    if !crate::platform::identity::has_identity() {
        return Access::NoIdentity;
    }

    let Ok(listener) = UserNotificationListener::Current() else {
        return Access::Unavailable;
    };

    // The blocking form: this is called off the UI thread, and the answer is
    // wanted before anything decides whether to start reading.
    let status = listener
        .RequestAccessAsync()
        .and_then(|operation| operation.get());

    match status {
        Ok(UserNotificationListenerAccessStatus::Allowed) => Access::Allowed,
        Ok(UserNotificationListenerAccessStatus::Denied) => Access::Denied,
        // `Unspecified` means the user has neither allowed nor refused.
        Ok(_) => Access::Unasked,
        Err(_) => Access::Unavailable,
    }
}

/// Whatever is in the Action Center now, as `(listener id, notification)`.
///
/// The listener reports everything currently there on every read, so the
/// caller is expected to put this through `bw_core::listener::Seen` rather
/// than posting it.
pub fn read() -> Result<Vec<(u32, NewNotification)>, String> {
    let listener = UserNotificationListener::Current()
        .map_err(|error| format!("the notification listener is not available: {error}"))?;

    let found = listener
        .GetNotificationsAsync(NotificationKinds::Toast)
        .and_then(|operation| operation.get())
        .map_err(|error| format!("could not read the notifications: {error}"))?;

    let mut out = Vec::new();
    for notification in found {
        let Ok(id) = notification.Id() else { continue };

        // An application that has since been uninstalled leaves notifications
        // whose `AppInfo` cannot be read. Skipping is right: there is nothing
        // to attribute them to.
        let (display_name, model_id) = match notification.AppInfo() {
            Ok(info) => (
                info.DisplayInfo()
                    .and_then(|display| display.DisplayName())
                    .map(|name| name.to_string_lossy())
                    .unwrap_or_default(),
                info.AppUserModelId()
                    .map(|id| id.to_string_lossy())
                    .unwrap_or_default(),
            ),
            Err(_) => (String::new(), String::new()),
        };

        let lines = text_of(&notification);
        let (summary, body) = listener::split_text(&lines);
        // A toast with no readable text is not something to show. It happens:
        // notifications carrying only an image, and ones whose binding is a
        // kind this does not ask for.
        if summary.is_empty() && body.is_empty() {
            continue;
        }

        out.push((
            id,
            NewNotification {
                app_name: listener::app_name(&display_name, &model_id),
                summary,
                body,
                ..NewNotification::default()
            },
        ));
    }

    Ok(out)
}

/// The text elements of a toast, in the order Windows lists them.
fn text_of(notification: &windows::UI::Notifications::UserNotification) -> Vec<String> {
    let Ok(binding) = notification
        .Notification()
        .and_then(|inner| inner.Visual())
        .and_then(|visual| {
            visual.GetBinding(&KnownNotificationBindings::ToastGeneric().unwrap_or_default())
        })
    else {
        return Vec::new();
    };

    let Ok(elements) = binding.GetTextElements() else {
        return Vec::new();
    };

    elements
        .into_iter()
        .filter_map(|element| element.Text().ok())
        .map(|text| text.to_string_lossy())
        .collect()
}
