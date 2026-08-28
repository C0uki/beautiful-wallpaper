//! The geometry and the bookkeeping of a screen capture.
//!
//! Three small pieces of arithmetic, each of which is wrong in a way that is
//! invisible until it is in front of someone: a rectangle dragged the wrong
//! way round, a device-independent bitmap whose rows are not padded, and a
//! file name Windows will not accept. All of it is pure, so it is tested here
//! rather than discovered on a machine none of this can be run on.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// What is done with the region once it has been chosen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum CaptureMode {
    /// Save it, and put it on the clipboard.
    Screenshot,
    /// Read the text out of it.
    Ocr,
    /// Read the text out of it, then translate it.
    Translate,
}

impl CaptureMode {
    /// Whether the result is an image on disk rather than text on screen.
    ///
    /// The two text modes deliberately save nothing: the product is what was
    /// read, and a folder slowly filling with crops of other people's windows
    /// is not something to do to someone by default.
    pub fn saves_a_file(self) -> bool {
        matches!(self, Self::Screenshot)
    }
}

/// A rectangle in whatever coordinate space the caller is working in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl Rect {
    /// The rectangle between two corners, whichever way round they came.
    ///
    /// Dragging up and to the left is exactly as normal as dragging down and
    /// to the right, and produces a negative width until this is applied.
    pub fn from_drag(start: (i32, i32), end: (i32, i32)) -> Self {
        let (x0, y0) = start;
        let (x1, y1) = end;
        Self {
            x: x0.min(x1),
            y: y0.min(y1),
            width: (x1 - x0).abs(),
            height: (y1 - y0).abs(),
        }
    }

    /// The part of this rectangle that is inside `bounds`.
    ///
    /// A drag that leaves the window reports coordinates outside it, and
    /// cropping to those would read past the end of the captured pixels.
    pub fn clamp(self, bounds: Self) -> Self {
        let left = self.x.max(bounds.x);
        let top = self.y.max(bounds.y);
        let right = (self.x + self.width).min(bounds.x + bounds.width);
        let bottom = (self.y + self.height).min(bounds.y + bounds.height);

        Self {
            x: left,
            y: top,
            width: (right - left).max(0),
            height: (bottom - top).max(0),
        }
    }

    /// Whether this is a selection rather than a click that moved slightly.
    ///
    /// Below this it is a cancel: nobody means to capture four pixels, and a
    /// launcher that produces a 2×3 image on a mis-click looks broken.
    pub fn is_usable(self) -> bool {
        self.width >= MIN_SELECTION && self.height >= MIN_SELECTION
    }

    /// This rectangle in the captured image's own pixels.
    ///
    /// The overlay works in CSS pixels and the capture is in physical ones, so
    /// everything the user drew has to be scaled. The edges round outwards:
    /// rounding both to nearest loses the last row of a selection about half
    /// the time, and a missing line of text is not a rounding error to the
    /// person who drew the box around it.
    pub fn to_physical(self, scale: f64) -> Self {
        let left = (f64::from(self.x) * scale).floor();
        let top = (f64::from(self.y) * scale).floor();
        let right = (f64::from(self.x + self.width) * scale).ceil();
        let bottom = (f64::from(self.y + self.height) * scale).ceil();

        Self {
            x: left as i32,
            y: top as i32,
            width: (right - left) as i32,
            height: (bottom - top) as i32,
        }
    }
}

/// The smallest selection worth acting on, in pixels.
const MIN_SELECTION: i32 = 8;

/// The length of one row of a device-independent bitmap, in bytes.
///
/// **Every row is padded to a four-byte boundary.** Handing the clipboard
/// unpadded rows does not fail: it produces an image that shears further to
/// one side with every row, which is one of the oldest bugs in Windows
/// graphics and looks like a corrupt file rather than a wrong number.
pub fn dib_stride(width: u32, bits_per_pixel: u32) -> usize {
    let bits = width as usize * bits_per_pixel as usize;
    // Round up to a whole number of four-byte groups.
    bits.div_ceil(32) * 4
}

/// The bytes a device-independent bitmap of this size occupies.
pub fn dib_size(width: u32, height: u32, bits_per_pixel: u32) -> usize {
    dib_stride(width, bits_per_pixel) * height as usize
}

/// The file name a screenshot is saved under.
///
/// `when` is formatted by the caller, which is the only part that needs a
/// clock; everything else about the name is fixed. Characters Windows refuses
/// are replaced rather than dropped, so two captures a second apart cannot
/// collide by having their differences deleted.
pub fn screenshot_name(when: &str) -> String {
    let stamp: String = when
        .chars()
        .map(|character| {
            if matches!(
                character,
                '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*'
            ) {
                '-'
            } else {
                character
            }
        })
        .collect();
    format!("Screenshot {}.png", stamp.trim())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: i32, y: i32, width: i32, height: i32) -> Rect {
        Rect {
            x,
            y,
            width,
            height,
        }
    }

    /// Dragging up and to the left is as ordinary as dragging the other way.
    #[test]
    fn a_drag_in_any_direction_gives_the_same_rectangle() {
        let expected = rect(10, 20, 90, 80);
        assert_eq!(Rect::from_drag((10, 20), (100, 100)), expected);
        assert_eq!(Rect::from_drag((100, 100), (10, 20)), expected);
        assert_eq!(Rect::from_drag((100, 20), (10, 100)), expected);
        assert_eq!(Rect::from_drag((10, 100), (100, 20)), expected);
    }

    #[test]
    fn a_drag_that_did_not_move_has_no_area() {
        let still = Rect::from_drag((50, 50), (50, 50));
        assert_eq!((still.width, still.height), (0, 0));
        assert!(!still.is_usable());
    }

    /// A drag that leaves the window reports coordinates outside it, and
    /// cropping to those reads past the end of the captured pixels.
    #[test]
    fn clamping_keeps_the_selection_inside_the_screen() {
        let screen = rect(0, 0, 1920, 1080);

        assert_eq!(
            rect(-50, -50, 200, 200).clamp(screen),
            rect(0, 0, 150, 150),
            "a selection that starts off the top-left"
        );
        assert_eq!(
            rect(1900, 1060, 200, 200).clamp(screen),
            rect(1900, 1060, 20, 20),
            "a selection that runs off the bottom-right"
        );
        assert_eq!(
            rect(100, 100, 200, 200).clamp(screen),
            rect(100, 100, 200, 200),
            "a selection that is already inside"
        );
    }

    #[test]
    fn a_selection_entirely_off_screen_clamps_to_nothing() {
        let screen = rect(0, 0, 1920, 1080);
        let away = rect(3000, 3000, 100, 100).clamp(screen);
        assert_eq!((away.width, away.height), (0, 0));
        assert!(!away.is_usable());
    }

    #[test]
    fn a_selection_smaller_than_a_mis_click_is_not_one() {
        assert!(!rect(0, 0, 4, 4).is_usable());
        assert!(!rect(0, 0, 200, 3).is_usable(), "a thin sliver counts too");
        assert!(rect(0, 0, 8, 8).is_usable());
    }

    /// Rounding to nearest loses the last row about half the time, and a
    /// missing line of text is not a rounding error to whoever drew the box.
    #[test]
    fn scaling_to_physical_pixels_rounds_outwards() {
        let selection = rect(10, 10, 100, 50);

        assert_eq!(selection.to_physical(1.0), selection);
        assert_eq!(selection.to_physical(2.0), rect(20, 20, 200, 100));

        let scaled = rect(10, 10, 101, 51).to_physical(1.5);
        assert_eq!(scaled.x, 15);
        assert_eq!(scaled.y, 15);
        // 10 + 101 = 111, times 1.5 is 166.5, which rounds up to 167.
        assert_eq!(scaled.width, 152);
        assert_eq!(scaled.height, 77);
    }

    #[test]
    fn scaling_never_shrinks_a_selection_below_what_was_drawn() {
        for scale in [1.0, 1.25, 1.5, 1.75, 2.0, 2.5, 3.0] {
            let scaled = rect(7, 13, 63, 29).to_physical(scale);
            assert!(
                f64::from(scaled.width) >= 63.0 * scale,
                "width shrank at {scale}"
            );
            assert!(
                f64::from(scaled.height) >= 29.0 * scale,
                "height shrank at {scale}"
            );
        }
    }

    /// The oldest bug in Windows graphics: rows that are not padded produce an
    /// image sheared a little further with every line.
    #[test]
    fn bitmap_rows_are_padded_to_four_bytes() {
        // Three bytes per pixel, so only widths that are a multiple of four
        // come out even.
        assert_eq!(dib_stride(4, 24), 12);
        assert_eq!(dib_stride(5, 24), 16, "15 bytes has to become 16");
        assert_eq!(dib_stride(6, 24), 20);
        assert_eq!(dib_stride(7, 24), 24);
        assert_eq!(dib_stride(8, 24), 24);

        // Four bytes per pixel is always aligned already.
        for width in 1..=16 {
            assert_eq!(dib_stride(width, 32), width as usize * 4);
        }
    }

    #[test]
    fn bitmap_size_counts_the_padding_on_every_row() {
        assert_eq!(dib_size(5, 3, 24), 48);
        assert_eq!(dib_size(1920, 1080, 24), 1920 * 3 * 1080);
    }

    #[test]
    fn a_screenshot_name_survives_what_windows_refuses() {
        assert_eq!(
            screenshot_name("2026-08-26 14:30:12"),
            "Screenshot 2026-08-26 14-30-12.png"
        );
        assert_eq!(
            screenshot_name("2026-08-26 143012"),
            "Screenshot 2026-08-26 143012.png"
        );
    }

    /// Two captures a second apart must not become the same name by having
    /// their differences deleted rather than replaced.
    #[test]
    fn two_screenshot_names_a_second_apart_stay_different() {
        let first = screenshot_name("2026-08-26 14:30:12");
        let second = screenshot_name("2026-08-26 14:30:13");
        assert_ne!(first, second);
    }

    #[test]
    fn only_a_screenshot_leaves_a_file_behind() {
        assert!(CaptureMode::Screenshot.saves_a_file());
        assert!(!CaptureMode::Ocr.saves_a_file());
        assert!(!CaptureMode::Translate.saves_a_file());
    }
}
