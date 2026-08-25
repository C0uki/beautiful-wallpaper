//! Small facts about the machine, for the sidebar's banner.
//!
//! Only the formatting lives here — reading the values is the shell crate's
//! job. It is a small amount of arithmetic, but it is the kind that is wrong
//! at exactly one boundary and right everywhere else, so it belongs where the
//! tests run.

/// Renders an uptime the way the original's banner does: the largest two units
/// that apply, and never a bare "0".
///
/// The original formats this in QML with `Math.floor` calls that produce
/// "0 minutes" for a machine that has just booted; this says "less than a
/// minute" instead, because a banner reading "Up • 0 minutes" looks broken.
pub fn format_uptime(seconds: u64) -> String {
    const MINUTE: u64 = 60;
    const HOUR: u64 = 60 * MINUTE;
    const DAY: u64 = 24 * HOUR;

    if seconds < MINUTE {
        return "less than a minute".to_owned();
    }

    let days = seconds / DAY;
    let hours = (seconds % DAY) / HOUR;
    let minutes = (seconds % HOUR) / MINUTE;

    let plural = |value: u64, unit: &str| {
        if value == 1 {
            format!("{value} {unit}")
        } else {
            format!("{value} {unit}s")
        }
    };

    // Two units is enough to be useful and short enough to fit the banner.
    // A third would push the line into eliding on a narrow sidebar.
    match (days, hours, minutes) {
        (0, 0, minutes) => plural(minutes, "minute"),
        (0, hours, 0) => plural(hours, "hour"),
        (0, hours, minutes) => format!("{}, {}", plural(hours, "hour"), plural(minutes, "minute")),
        (days, 0, _) => plural(days, "day"),
        (days, hours, _) => format!("{}, {}", plural(days, "day"), plural(hours, "hour")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_machine_that_just_booted_does_not_read_as_zero() {
        assert_eq!(format_uptime(0), "less than a minute");
        assert_eq!(format_uptime(59), "less than a minute");
    }

    #[test]
    fn singular_units_are_not_pluralised() {
        assert_eq!(format_uptime(60), "1 minute");
        assert_eq!(format_uptime(3600), "1 hour");
        assert_eq!(format_uptime(86_400), "1 day");
    }

    #[test]
    fn two_units_are_shown_when_both_are_nonzero() {
        assert_eq!(format_uptime(3600 + 120), "1 hour, 2 minutes");
        assert_eq!(format_uptime(86_400 + 7200), "1 day, 2 hours");
    }

    #[test]
    fn a_zero_middle_unit_is_dropped_rather_than_printed() {
        // "2 hours, 0 minutes" is the kind of thing that makes a banner look
        // machine-generated.
        assert_eq!(format_uptime(7200), "2 hours");
        assert_eq!(format_uptime(86_400 * 3), "3 days");
    }

    #[test]
    fn minutes_are_never_shown_next_to_days() {
        // Nobody reading "up 4 days" cares about the minutes, and the line has
        // to fit a narrow sidebar.
        let text = format_uptime(86_400 * 4 + 3600 * 5 + 90);
        assert_eq!(text, "4 days, 5 hours");
        assert!(!text.contains("minute"));
    }

    #[test]
    fn a_long_uptime_still_formats() {
        assert_eq!(format_uptime(86_400 * 400 + 3600), "400 days, 1 hour");
    }
}
