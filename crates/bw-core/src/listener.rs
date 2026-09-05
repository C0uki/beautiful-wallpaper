//! Turning Windows' own notifications into the shell's.
//!
//! The original has nothing like this: on Linux the shell *is* the
//! notification server, so every notification arrives once, addressed to it.
//! Windows keeps its own Action Center and hands a reader the whole of it,
//! which inverts the problem — the question stops being "what arrived" and
//! becomes "what is new since I last looked".
//!
//! Two things follow, and both are silent when got wrong:
//!
//! **Every read returns everything.** `UserNotificationListener` reports what
//! is *currently* in the Action Center, so feeding a read straight into the
//! store would repost every notification on every change. Ids are what
//! separates a new notification from one that is merely still there.
//!
//! **The first read is not an arrival.** The Action Center holds everything
//! the user has not dismissed, which is often days of it. A shell that
//! replayed all of that the moment somebody switched the feature on would bury
//! whatever was actually happening under a week of history.
//!
//! Neither needs Windows to reproduce, so both are here under tests.

use std::collections::BTreeSet;

/// Which notifications have already been passed on.
#[derive(Debug, Default)]
pub struct Seen {
    /// Listener ids currently in the Action Center, as of the last look.
    ids: BTreeSet<u32>,
    /// Whether anything has been looked at yet.
    primed: bool,
}

impl Seen {
    pub fn new() -> Self {
        Self::default()
    }

    /// The ids that are new since the last look.
    ///
    /// The first call returns nothing and simply records what is there: see
    /// the note above about replaying a week of history. Ids that have gone
    /// are forgotten, so this does not grow without bound over a long session
    /// — and a notification the user dismissed and the application posted
    /// again arrives with a new id, so forgetting is also what makes the
    /// second one count as new.
    pub fn arrivals(&mut self, present: &[u32]) -> Vec<u32> {
        let now: BTreeSet<u32> = present.iter().copied().collect();

        let new = if self.primed {
            present
                .iter()
                .copied()
                .filter(|id| !self.ids.contains(id))
                .collect()
        } else {
            self.primed = true;
            Vec::new()
        };

        self.ids = now;
        new
    }

    /// Whether anything has been looked at yet.
    pub fn is_primed(&self) -> bool {
        self.primed
    }

    /// Forgets everything, so the next look primes again.
    ///
    /// Used when the listener is switched off and on: what is in the Action
    /// Center while nobody is reading it is not an arrival either.
    pub fn reset(&mut self) {
        self.ids.clear();
        self.primed = false;
    }
}

/// A toast's text as a summary and a body.
///
/// Windows hands over a list of text elements with nothing marking which is
/// which. By the `ToastGeneric` convention the first is the title and the rest
/// are the body, which is what this assumes — but plenty of applications post
/// a single line, and a few post an empty first element.
///
/// The lines are joined with newlines rather than spaces: they are separate
/// lines because the application made them separate, and running them together
/// turns a two-line message into one long one.
pub fn split_text(lines: &[impl AsRef<str>]) -> (String, String) {
    let mut text = lines
        .iter()
        .map(|line| line.as_ref().trim())
        .filter(|line| !line.is_empty());

    let summary = text.next().unwrap_or_default().to_owned();
    let body = text.collect::<Vec<_>>().join("\n");
    (summary, body)
}

/// What to show as the sender.
///
/// The display name is what a person recognises. It can be empty — a packaged
/// application with no display name in its manifest, or one the shell could
/// not read — and the model id is the fallback rather than a blank line,
/// because "something posted this and I cannot tell you what" is worse than an
/// ugly identifier.
pub fn app_name(display_name: &str, model_id: &str) -> String {
    let display = display_name.trim();
    if !display.is_empty() {
        return display.to_owned();
    }

    let model = model_id.trim();
    if model.is_empty() {
        return "Windows".to_owned();
    }

    // A model id is `Company.Product_publisherhash!App` or a path-like string
    // for a desktop application. The last readable segment is the closest
    // thing to a name it carries.
    model
        .rsplit(['!', '\\'])
        .find(|part| !part.trim().is_empty())
        .unwrap_or(model)
        .trim()
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The failure this exists for: switching the feature on would otherwise
    /// replay everything the user had not dismissed, all at once.
    #[test]
    fn the_first_look_reports_nothing() {
        let mut seen = Seen::new();
        assert!(!seen.is_primed());

        assert_eq!(seen.arrivals(&[1, 2, 3]), Vec::<u32>::new());
        assert!(seen.is_primed());
    }

    #[test]
    fn only_what_was_not_there_before_is_an_arrival() {
        let mut seen = Seen::new();
        seen.arrivals(&[1, 2]);

        assert_eq!(seen.arrivals(&[1, 2, 3]), vec![3]);
        // Still there is not an arrival, however many times it is read.
        assert_eq!(seen.arrivals(&[1, 2, 3]), Vec::<u32>::new());
        assert_eq!(seen.arrivals(&[1, 2, 3]), Vec::<u32>::new());
    }

    #[test]
    fn arrivals_keep_the_order_windows_reported_them_in() {
        let mut seen = Seen::new();
        seen.arrivals(&[1]);
        assert_eq!(seen.arrivals(&[1, 7, 4, 9]), vec![7, 4, 9]);
    }

    /// Dismissing everything and having one arrive again must report it, which
    /// is what forgetting departed ids is for — and it is also what keeps the
    /// set from growing for the life of the session.
    #[test]
    fn an_id_that_has_gone_is_forgotten() {
        let mut seen = Seen::new();
        seen.arrivals(&[1, 2, 3]);
        assert_eq!(seen.arrivals(&[]), Vec::<u32>::new());

        // The same ids again: the listener reuses them only after the
        // originals are gone, and either way these are new to the shell now.
        assert_eq!(seen.arrivals(&[1, 2]), vec![1, 2]);
    }

    /// What is in the Action Center while nobody is reading is not an arrival
    /// either, so switching the listener off and on primes again.
    #[test]
    fn resetting_primes_again() {
        let mut seen = Seen::new();
        seen.arrivals(&[1]);
        seen.reset();

        assert!(!seen.is_primed());
        assert_eq!(seen.arrivals(&[1, 2]), Vec::<u32>::new());
        assert_eq!(seen.arrivals(&[1, 2, 3]), vec![3]);
    }

    #[test]
    fn the_first_line_is_the_summary_and_the_rest_is_the_body() {
        let (summary, body) = split_text(&["Kagami", "Are you coming?", "It starts at six"]);
        assert_eq!(summary, "Kagami");
        assert_eq!(body, "Are you coming?\nIt starts at six");
    }

    #[test]
    fn a_single_line_is_all_summary_and_no_body() {
        let (summary, body) = split_text(&["Build finished"]);
        assert_eq!(summary, "Build finished");
        assert_eq!(body, "");
    }

    /// Applications post empty elements, and a blank summary with the real
    /// text in the body reads as a notification from nobody about nothing.
    #[test]
    fn empty_elements_are_stepped_over() {
        let (summary, body) = split_text(&["", "  ", "Actually the title", "and the body"]);
        assert_eq!(summary, "Actually the title");
        assert_eq!(body, "and the body");

        let empty: [&str; 0] = [];
        let (summary, body) = split_text(&empty);
        assert_eq!(summary, "");
        assert_eq!(body, "");
    }

    #[test]
    fn the_sender_is_the_display_name_when_there_is_one() {
        assert_eq!(app_name("Slack", "com.slack.Slack_x!App"), "Slack");
        assert_eq!(app_name("  Mail  ", ""), "Mail");
    }

    /// "Something posted this and I cannot tell you what" is worse than an
    /// ugly identifier.
    #[test]
    fn without_a_display_name_the_model_id_stands_in() {
        assert_eq!(
            app_name("", "Microsoft.WindowsTerminal_8wekyb3d8bbwe!App"),
            "App"
        );
        assert_eq!(
            app_name("", r"C:\Program Files\thing\thing.exe"),
            "thing.exe"
        );
        assert_eq!(app_name("", "SomeCompany.SomeApp"), "SomeCompany.SomeApp");
        assert_eq!(app_name("", "   "), "Windows");
        assert_eq!(app_name("", ""), "Windows");
    }
}
