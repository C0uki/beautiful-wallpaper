//! Reading a Valorant crosshair code.
//!
//! The overlay's crosshair is configured by pasting the share code the game
//! itself produces — `0;P;c;1;h;0;0l;4;0o;2;0a;1;1b;0` — which is what makes it
//! useful: people already have one they like, from the game or from a builder
//! site, and typing twenty numbers into a config file is not the same offer.
//!
//! The format is a flat `key;value;key;value` list with two-character keys and
//! no schema. Three things about it are easy to get wrong, and all three are
//! silent:
//!
//! * **A code is a patch, not a document.** It carries only what differs from
//!   the game's defaults, so the defaults have to be laid down first or a code
//!   that sets one field resets every other to zero.
//! * **Keys nobody knows have to be skipped**, not treated as an error. Every
//!   real code starts `0;P` — a profile marker this does not model — and codes
//!   from newer builds carry fields that did not exist when this was written.
//! * **The unbind flags are applied last.** `0v` sets the vertical inner
//!   length, but it only means anything when `0g` says the axes are unbound;
//!   otherwise the vertical length follows the horizontal one, whatever `0v`
//!   said and whichever order the two arrived in.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// The game's own defaults, which every code is a patch on top of.
const DEFAULT_CODE: &str = "c;0;u;FFFFFF;h;1;o;0.5;t;1;d;0;a;1;z;2;\
     0b;1;0a;0.8;0l;6;0v;6;0g;0;0t;2;0o;3;\
     1b;1;1a;0.35;1l;2;1v;2;1g;0;1t;2;1o;10";

/// The eight colours the game offers by index; index 8 means "use the hex".
const PALETTE: [&str; 8] = [
    "#FFFFFF", "#00FF00", "#7FFF00", "#DFFF00", "#FFFF00", "#00FFFF", "#FF00FF", "#FF0000",
];

/// A crosshair, ready to draw.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Crosshair {
    /// The resolved colour, as CSS.
    pub color: String,
    pub outline: bool,
    pub outline_opacity: f64,
    pub outline_thickness: u32,
    pub center_dot: bool,
    pub center_dot_opacity: f64,
    pub center_dot_size: u32,
    pub inner_lines: bool,
    pub inner_line_opacity: f64,
    pub inner_line_length: u32,
    pub inner_line_vertical_length: u32,
    pub inner_line_thickness: u32,
    pub inner_line_offset: u32,
    pub outer_lines: bool,
    pub outer_line_opacity: f64,
    pub outer_line_length: u32,
    pub outer_line_vertical_length: u32,
    pub outer_line_thickness: u32,
    pub outer_line_offset: u32,
    /// The side of the square the whole thing needs, in pixels.
    ///
    /// Worked out here rather than measured after drawing: the widget has to
    /// be centred on a point, and a box that is sized by its contents would
    /// move the centre every time a line got longer.
    pub size: u32,
}

/// Everything a code can set, before the derived parts are worked out.
#[derive(Debug, Clone)]
struct Raw {
    color: u32,
    color_code: String,
    outline: bool,
    outline_opacity: f64,
    outline_thickness: u32,
    center_dot: bool,
    center_dot_opacity: f64,
    center_dot_size: u32,
    inner_lines: bool,
    inner_line_opacity: f64,
    inner_line_length: u32,
    inner_line_vertical_length: u32,
    inner_line_unbind: bool,
    inner_line_thickness: u32,
    inner_line_offset: u32,
    outer_lines: bool,
    outer_line_opacity: f64,
    outer_line_length: u32,
    outer_line_vertical_length: u32,
    outer_line_unbind: bool,
    outer_line_thickness: u32,
    outer_line_offset: u32,
}

impl Default for Raw {
    /// Zeroes, not sensible values: the real defaults come from parsing
    /// [`DEFAULT_CODE`], so there is one place they are written down.
    fn default() -> Self {
        Self {
            color: 0,
            color_code: String::from("#FFFFFF"),
            outline: false,
            outline_opacity: 0.0,
            outline_thickness: 0,
            center_dot: false,
            center_dot_opacity: 0.0,
            center_dot_size: 0,
            inner_lines: false,
            inner_line_opacity: 0.0,
            inner_line_length: 0,
            inner_line_vertical_length: 0,
            inner_line_unbind: false,
            inner_line_thickness: 0,
            inner_line_offset: 0,
            outer_lines: false,
            outer_line_opacity: 0.0,
            outer_line_length: 0,
            outer_line_vertical_length: 0,
            outer_line_unbind: false,
            outer_line_thickness: 0,
            outer_line_offset: 0,
        }
    }
}

/// Reads a share code into something drawable.
///
/// Never fails. A code is pasted by hand from a game or a website, and the
/// only useful answer to a malformed one is the crosshair the readable part of
/// it describes — refusing to draw anything would leave the user with a blank
/// square and no idea which character was wrong.
pub fn parse(code: &str) -> Crosshair {
    let mut raw = Raw::default();
    apply(&mut raw, DEFAULT_CODE);
    apply(&mut raw, code);

    // Last, and after both codes: a vertical length only means anything when
    // the axes are unbound, whichever order the two fields arrived in.
    if !raw.inner_line_unbind {
        raw.inner_line_vertical_length = raw.inner_line_length;
    }
    if !raw.outer_line_unbind {
        raw.outer_line_vertical_length = raw.outer_line_length;
    }

    finish(raw)
}

/// Applies one code's key/value pairs over what is already there.
fn apply(raw: &mut Raw, code: &str) {
    let fields: Vec<&str> = code.split(';').map(str::trim).collect();

    // Stepping in twos, and stopping before a key with no value: a truncated
    // code should give up its last field, not the whole crosshair.
    for pair in fields.chunks_exact(2) {
        let (key, value) = (pair[0], pair[1]);
        match key {
            "c" => raw.color = number(value).unwrap_or(raw.color as f64) as u32,
            "u" => raw.color_code = hex(value),
            "h" => raw.outline = flag(value),
            "o" => raw.outline_opacity = opacity(value, raw.outline_opacity),
            "t" => raw.outline_thickness = pixels(value, raw.outline_thickness),
            "d" => raw.center_dot = flag(value),
            "a" => raw.center_dot_opacity = opacity(value, raw.center_dot_opacity),
            "z" => raw.center_dot_size = pixels(value, raw.center_dot_size),
            "0b" => raw.inner_lines = flag(value),
            "0a" => raw.inner_line_opacity = opacity(value, raw.inner_line_opacity),
            "0l" => raw.inner_line_length = pixels(value, raw.inner_line_length),
            "0v" => {
                raw.inner_line_vertical_length = pixels(value, raw.inner_line_vertical_length);
            }
            "0g" => raw.inner_line_unbind = flag(value),
            "0t" => raw.inner_line_thickness = pixels(value, raw.inner_line_thickness),
            "0o" => raw.inner_line_offset = pixels(value, raw.inner_line_offset),
            "1b" => raw.outer_lines = flag(value),
            "1a" => raw.outer_line_opacity = opacity(value, raw.outer_line_opacity),
            "1l" => raw.outer_line_length = pixels(value, raw.outer_line_length),
            "1v" => {
                raw.outer_line_vertical_length = pixels(value, raw.outer_line_vertical_length);
            }
            "1g" => raw.outer_line_unbind = flag(value),
            "1t" => raw.outer_line_thickness = pixels(value, raw.outer_line_thickness),
            "1o" => raw.outer_line_offset = pixels(value, raw.outer_line_offset),
            // Every real code opens with a profile marker, and newer ones
            // carry fields this does not model. Neither is a reason to stop.
            _ => {}
        }
    }
}

/// The derived measurements, which is everything the drawing needs.
fn finish(raw: Raw) -> Crosshair {
    let border = if raw.outline {
        raw.outline_thickness
    } else {
        0
    };
    // Half the dot, plus a pixel, is where a line is allowed to start.
    let from_centre = raw.center_dot_size / 2 + 1;
    let inner_offset = from_centre + raw.inner_line_offset;
    let outer_offset = from_centre + raw.outer_line_offset;

    let dot_span = raw.center_dot_size + border * 2;
    let inner_span = (inner_offset + raw.inner_line_length + border) * 2;
    let outer_span = (outer_offset + raw.outer_line_length + border) * 2;

    Crosshair {
        color: if raw.color == 8 {
            raw.color_code.clone()
        } else {
            PALETTE
                .get(raw.color as usize)
                .copied()
                .unwrap_or("#FFFFFF")
                .to_owned()
        },
        outline: raw.outline,
        outline_opacity: raw.outline_opacity,
        outline_thickness: raw.outline_thickness,
        center_dot: raw.center_dot,
        center_dot_opacity: raw.center_dot_opacity,
        center_dot_size: raw.center_dot_size,
        inner_lines: raw.inner_lines,
        inner_line_opacity: raw.inner_line_opacity,
        inner_line_length: raw.inner_line_length,
        inner_line_vertical_length: raw.inner_line_vertical_length,
        inner_line_thickness: raw.inner_line_thickness,
        inner_line_offset: inner_offset,
        outer_lines: raw.outer_lines,
        outer_line_opacity: raw.outer_line_opacity,
        outer_line_length: raw.outer_line_length,
        outer_line_vertical_length: raw.outer_line_vertical_length,
        outer_line_thickness: raw.outer_line_thickness,
        outer_line_offset: outer_offset,
        // The two is the original's pixel correction, kept so a crosshair
        // built on a share-code site lands on the same size here.
        size: dot_span.max(inner_span).max(outer_span) + 2,
    }
}

fn number(value: &str) -> Option<f64> {
    value.parse::<f64>().ok().filter(|found| found.is_finite())
}

fn flag(value: &str) -> bool {
    value == "1"
}

fn opacity(value: &str, current: f64) -> f64 {
    number(value).map_or(current, |found| found.clamp(0.0, 1.0))
}

/// A pixel measurement. Negative is meaningless and would invert a rectangle.
fn pixels(value: &str, current: u32) -> u32 {
    number(value).map_or(current, |found| found.max(0.0).min(4096.0) as u32)
}

/// The colour field, which carries six hex digits and sometimes more.
fn hex(value: &str) -> String {
    let digits: String = value
        .chars()
        .filter(|character| character.is_ascii_hexdigit())
        .take(6)
        .collect();
    if digits.len() == 6 {
        format!("#{digits}")
    } else {
        String::from("#FFFFFF")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The config's own default, so the shipped crosshair is a tested one.
    const SHIPPED: &str = "0;P;d;1;0l;10;0o;2;1b;0";

    #[test]
    fn a_code_is_a_patch_on_the_games_defaults() {
        let found = parse(SHIPPED);

        // What the code said.
        assert!(found.center_dot, "d;1");
        assert_eq!(found.inner_line_length, 10, "0l;10");
        assert!(!found.outer_lines, "1b;0");

        // What it did not say, which must still be the game's defaults rather
        // than zero: a code carries only the differences.
        assert!(found.outline, "h defaults to on");
        assert_eq!(found.outline_thickness, 1);
        assert_eq!(found.inner_line_thickness, 2);
        assert_eq!(found.center_dot_size, 2);
        assert!((found.inner_line_opacity - 0.8).abs() < f64::EPSILON);
    }

    /// Every real code opens `0;P`, which is a profile marker this does not
    /// model. Treating an unknown key as an error would reject every code.
    #[test]
    fn unknown_keys_are_stepped_over() {
        let with_marker = parse("0;P;d;1");
        let without = parse("d;1");
        assert_eq!(with_marker, without);
    }

    #[test]
    fn a_code_from_a_newer_build_still_parses() {
        let found = parse("0;P;d;1;zz;9;yy;whatever;0l;7");
        assert!(found.center_dot);
        assert_eq!(found.inner_line_length, 7);
    }

    /// The trap: a vertical length is only meaningful when the axes are
    /// unbound, whichever order the two fields arrive in.
    #[test]
    fn a_vertical_length_is_ignored_until_the_axes_are_unbound() {
        let bound = parse("0l;6;0v;20");
        assert_eq!(
            bound.inner_line_vertical_length, 6,
            "0v means nothing while the axes are bound"
        );

        let unbound = parse("0l;6;0v;20;0g;1");
        assert_eq!(unbound.inner_line_vertical_length, 20);

        // And the same when the unbind flag comes first, which is the order
        // a naive left-to-right parse gets wrong.
        let reordered = parse("0g;1;0v;20;0l;6");
        assert_eq!(reordered.inner_line_vertical_length, 20);
    }

    #[test]
    fn the_outer_lines_have_the_same_rule() {
        assert_eq!(parse("1l;4;1v;9").outer_line_vertical_length, 4);
        assert_eq!(parse("1l;4;1v;9;1g;1").outer_line_vertical_length, 9);
    }

    #[test]
    fn a_colour_index_picks_from_the_games_palette() {
        assert_eq!(parse("c;0").color, "#FFFFFF");
        assert_eq!(parse("c;1").color, "#00FF00");
        assert_eq!(parse("c;7").color, "#FF0000");
    }

    /// Index eight is the one that means "and here is the hex".
    #[test]
    fn index_eight_uses_the_custom_colour() {
        assert_eq!(parse("c;8;u;FF8800").color, "#FF8800");
        // Codes carry eight digits when there is an alpha; the crosshair's
        // opacity is a separate field, so only the six are taken.
        assert_eq!(parse("c;8;u;FF8800FF").color, "#FF8800");
    }

    #[test]
    fn a_colour_index_nobody_has_heard_of_falls_back_to_white() {
        assert_eq!(parse("c;42").color, "#FFFFFF");
        assert_eq!(parse("c;8;u;nonsense").color, "#FFFFFF");
    }

    /// A code pasted from a chat window can lose its tail.
    #[test]
    fn a_truncated_code_gives_up_only_its_last_field() {
        let found = parse("0;P;d;1;0l");
        assert!(found.center_dot, "the complete pairs still count");
        assert_eq!(found.inner_line_length, 6, "the dangling key is dropped");
    }

    #[test]
    fn an_empty_code_is_the_games_default_crosshair() {
        let empty = parse("");
        assert!(empty.inner_lines);
        assert_eq!(empty.inner_line_length, 6);
        assert_eq!(empty.color, "#FFFFFF");
    }

    /// Nonsense must not become a rectangle with a negative side.
    #[test]
    fn negative_and_unreadable_numbers_never_reach_the_drawing() {
        let found = parse("0l;-40;0t;-2;o;-1;a;5");
        assert_eq!(found.inner_line_length, 0);
        assert_eq!(found.inner_line_thickness, 0);
        assert!((0.0..=1.0).contains(&found.outline_opacity));
        assert!((0.0..=1.0).contains(&found.center_dot_opacity));

        let unreadable = parse("0l;banana");
        assert_eq!(
            unreadable.inner_line_length, 6,
            "an unreadable number leaves the previous value alone"
        );
    }

    /// The widget is centred on a point, so its box has to be square and big
    /// enough for the longest thing in it.
    #[test]
    fn the_size_covers_the_longest_line() {
        let long = parse("0g;0;0l;40;0o;5;1b;0;d;0;z;2;h;0");
        // (2 / 2 + 1 + 5 + 40 + 0) * 2 + 2
        assert_eq!(long.size, (1 + 1 + 5 + 40) * 2 + 2);

        // Outer lines further out than the inner ones decide the size.
        let outer = parse("0l;2;0o;1;1b;1;1l;3;1o;30;h;0;z;0");
        assert!(outer.size >= (0 + 1 + 30 + 3) * 2);
    }

    #[test]
    fn the_size_is_never_zero_even_with_everything_switched_off() {
        let nothing = parse("d;0;0b;0;1b;0;h;0;z;0;0l;0;1l;0;0o;0;1o;0");
        assert!(nothing.size >= 2);
    }

    /// The offsets that come out are the ones a drawing can use directly —
    /// measured from the centre, not from the edge of the dot.
    #[test]
    fn the_offsets_are_measured_from_the_centre() {
        let found = parse("z;4;0o;3;1o;10");
        assert_eq!(found.inner_line_offset, 4 / 2 + 1 + 3);
        assert_eq!(found.outer_line_offset, 4 / 2 + 1 + 10);
    }
}
