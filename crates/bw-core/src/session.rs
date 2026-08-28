//! Which way out of the session the machine will actually offer.
//!
//! Six buttons, of which a given machine can rarely do all six, and two of
//! which end the session the instant they are pressed. Both of those facts are
//! decisions rather than mechanics — what to hide, what order to put the rest
//! in, and what the keyboard is pointing at when the screen opens — so they
//! live here under tests rather than in the Win32 half.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::config::Session;

/// One way out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum SessionAction {
    /// The screen, not the session: everything stays running.
    Lock,
    Sleep,
    Hibernate,
    LogOut,
    Restart,
    ShutDown,
}

impl SessionAction {
    /// Every action, least drastic first.
    ///
    /// The order is fixed rather than configurable, and it is the order the
    /// buttons are drawn in. Somebody reaching for "lock" should never find
    /// "shut down" where they expected it because a config file was edited.
    pub const ORDER: [Self; 6] = [
        Self::Lock,
        Self::Sleep,
        Self::Hibernate,
        Self::LogOut,
        Self::Restart,
        Self::ShutDown,
    ];

    /// What this is called in the CLI and the launcher.
    pub fn keyword(self) -> &'static str {
        match self {
            Self::Lock => "lock",
            Self::Sleep => "sleep",
            Self::Hibernate => "hibernate",
            Self::LogOut => "logout",
            Self::Restart => "restart",
            Self::ShutDown => "shutdown",
        }
    }

    pub fn symbol(self) -> &'static str {
        match self {
            Self::Lock => "lock",
            Self::Sleep => "bedtime",
            Self::Hibernate => "ac_unit",
            Self::LogOut => "logout",
            Self::Restart => "restart_alt",
            Self::ShutDown => "power_settings_new",
        }
    }

    /// Whether taking this closes the user's programs.
    ///
    /// Sleeping and hibernating are not on this list: the machine comes back
    /// with everything where it was left. Locking is not either.
    pub fn ends_the_session(self) -> bool {
        matches!(self, Self::LogOut | Self::Restart | Self::ShutDown)
    }

    pub fn from_keyword(keyword: &str) -> Option<Self> {
        let wanted = keyword.trim().to_lowercase();
        Self::ORDER
            .into_iter()
            .find(|action| action.keyword() == wanted)
    }
}

/// What the machine's firmware and power configuration allow.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PowerCapabilities {
    /// One of the old S1–S3 sleep states is available.
    pub standby: bool,
    /// The machine uses modern standby (S0 low power idle) instead.
    pub modern_standby: bool,
    /// A hibernation file exists. Without one there is nowhere to write to.
    pub hibernate_file: bool,
}

impl PowerCapabilities {
    /// **Not just S3.** Most machines built in the last several years report
    /// every one of S1–S3 as unavailable and sleep through modern standby
    /// instead, so checking S3 alone hides the sleep button on exactly the
    /// hardware that sleeps best.
    pub fn can_sleep(self) -> bool {
        self.standby || self.modern_standby
    }

    pub fn can_hibernate(self) -> bool {
        self.hibernate_file
    }
}

/// The buttons to draw, in the order to draw them.
///
/// Filtered by what the user asked for *and* by what the machine can do: an
/// action that cannot work is left out rather than shown and then found dead,
/// which is how brightness and text recognition are handled elsewhere.
pub fn available(config: &Session, capabilities: PowerCapabilities) -> Vec<SessionAction> {
    SessionAction::ORDER
        .into_iter()
        .filter(|action| match action {
            SessionAction::Lock => config.lock,
            SessionAction::Sleep => config.sleep && capabilities.can_sleep(),
            SessionAction::Hibernate => config.hibernate && capabilities.can_hibernate(),
            SessionAction::LogOut => config.log_out,
            SessionAction::Restart => config.restart,
            SessionAction::ShutDown => config.shut_down,
        })
        .collect()
}

/// Which button the keyboard starts on, if any.
///
/// **Never one that ends the session.** The screen opens under a key the user
/// pressed, and Enter is one keystroke further; starting on "shut down" turns
/// a mistyped shortcut into a lost afternoon. If nothing harmless is on
/// offer, nothing is focused and the user has to say which they meant.
pub fn initial_focus(actions: &[SessionAction]) -> Option<usize> {
    actions.iter().position(|action| !action.ends_the_session())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn everything() -> Session {
        Session::default()
    }

    fn capable() -> PowerCapabilities {
        PowerCapabilities {
            standby: true,
            modern_standby: false,
            hibernate_file: true,
        }
    }

    /// Checking S3 alone hides sleep on exactly the hardware that sleeps best.
    #[test]
    fn modern_standby_counts_as_being_able_to_sleep() {
        let modern = PowerCapabilities {
            standby: false,
            modern_standby: true,
            hibernate_file: false,
        };
        assert!(modern.can_sleep());
        assert!(!modern.can_hibernate());

        let old = PowerCapabilities {
            standby: true,
            modern_standby: false,
            hibernate_file: false,
        };
        assert!(old.can_sleep());

        assert!(!PowerCapabilities::default().can_sleep());
    }

    #[test]
    fn the_buttons_come_out_least_drastic_first() {
        let actions = available(&everything(), capable());
        assert_eq!(actions, SessionAction::ORDER.to_vec());
    }

    /// An action that cannot work is left out rather than shown and found dead.
    #[test]
    fn what_the_machine_cannot_do_is_not_offered() {
        let none = PowerCapabilities::default();
        let actions = available(&everything(), none);

        assert!(!actions.contains(&SessionAction::Sleep));
        assert!(!actions.contains(&SessionAction::Hibernate));
        assert!(actions.contains(&SessionAction::Lock));
        assert!(actions.contains(&SessionAction::ShutDown));
    }

    #[test]
    fn hibernating_needs_somewhere_to_write_to() {
        let no_file = PowerCapabilities {
            standby: true,
            modern_standby: false,
            hibernate_file: false,
        };
        let actions = available(&everything(), no_file);
        assert!(actions.contains(&SessionAction::Sleep));
        assert!(!actions.contains(&SessionAction::Hibernate));
    }

    #[test]
    fn switching_one_off_in_the_config_removes_it() {
        let mut config = everything();
        config.restart = false;
        config.lock = false;

        let actions = available(&config, capable());
        assert!(!actions.contains(&SessionAction::Restart));
        assert!(!actions.contains(&SessionAction::Lock));
        assert!(actions.contains(&SessionAction::ShutDown));
    }

    /// The screen opens under a key, and Enter is one keystroke further.
    #[test]
    fn the_keyboard_never_starts_on_something_that_ends_the_session() {
        for capabilities in [capable(), PowerCapabilities::default()] {
            let mut config = everything();

            // Whatever is switched off, whatever the machine can do, the
            // starting button must be a recoverable one.
            for lock in [true, false] {
                config.lock = lock;
                let actions = available(&config, capabilities);
                let Some(index) = initial_focus(&actions) else {
                    continue;
                };
                assert!(
                    !actions[index].ends_the_session(),
                    "started on {:?}",
                    actions[index]
                );
            }
        }
    }

    #[test]
    fn with_nothing_harmless_on_offer_nothing_is_focused() {
        let mut config = everything();
        config.lock = false;
        config.sleep = false;
        config.hibernate = false;

        let actions = available(&config, capable());
        assert!(!actions.is_empty());
        assert_eq!(initial_focus(&actions), None);
    }

    #[test]
    fn keywords_round_trip() {
        for action in SessionAction::ORDER {
            assert_eq!(SessionAction::from_keyword(action.keyword()), Some(action));
        }
    }

    /// These arrive from a command line and from a search box, so the casing
    /// and the spacing are whatever the user typed.
    #[test]
    fn a_keyword_is_read_the_way_it_was_typed() {
        assert_eq!(
            SessionAction::from_keyword(" ShutDown "),
            Some(SessionAction::ShutDown)
        );
        assert_eq!(
            SessionAction::from_keyword("SLEEP"),
            Some(SessionAction::Sleep)
        );
        assert_eq!(SessionAction::from_keyword("explode"), None);
        assert_eq!(SessionAction::from_keyword(""), None);
    }

    #[test]
    fn sleeping_and_locking_do_not_close_anything() {
        assert!(!SessionAction::Lock.ends_the_session());
        assert!(!SessionAction::Sleep.ends_the_session());
        assert!(!SessionAction::Hibernate.ends_the_session());
        assert!(SessionAction::LogOut.ends_the_session());
        assert!(SessionAction::Restart.ends_the_session());
        assert!(SessionAction::ShutDown.ends_the_session());
    }
}
