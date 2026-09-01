//! The screen's own chrome: fake rounded corners, a frame, and hot corners.
//!
//! Three decorations that all live at the very edge of the display, and all
//! three go wrong in the same direction if the arithmetic is done carelessly —
//! anchored to the wrong corner, or overlapping each other, in a place where
//! nothing else is drawn to make the mistake obvious.
//!
//! The hot corners are the part with teeth. They are input regions the user
//! cannot see, so a region that is a few pixels off is not a cosmetic bug: it
//! is a sidebar that opens when somebody reaches for a window's close button.
//! Everything about where they are is decided here, under tests.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::capture::Rect;
use crate::config::{Bar, CornerOpen};

/// When the fake screen rounding is drawn.
///
/// The middle case is the default and the reason the whole thing is not just a
/// boolean: rounded corners over a full-screen video are four black notches
/// cut out of the picture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum FakeRounding {
    Never,
    Always,
    WhenNotFullscreen,
}

impl FakeRounding {
    /// Reads the config's number, which is the original's vocabulary.
    ///
    /// Anything unrecognised falls back to the default rather than to
    /// `Never`: a typo in the config should not make a feature disappear with
    /// no explanation.
    pub fn from_config(value: u32) -> Self {
        match value {
            0 => Self::Never,
            1 => Self::Always,
            _ => Self::WhenNotFullscreen,
        }
    }

    pub fn shows(self, fullscreen: bool) -> bool {
        match self {
            Self::Never => false,
            Self::Always => true,
            Self::WhenNotFullscreen => !fullscreen,
        }
    }
}

/// Everything the chrome surface needs, already decided.
///
/// Resolved here rather than in the surface so the policy exists once. The
/// alternative is a three-line rule about full-screen windows written in Rust
/// and again in TypeScript, which is two rules the first time either is
/// touched.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ScreenChrome {
    /// Whether the fake rounded corners are drawn right now.
    pub corners_visible: bool,
    /// Their radius, in pixels.
    pub radius: u32,
    /// The edges the frame is drawn on. Empty when there is no frame.
    pub frame_edges: Vec<Edge>,
    pub frame_thickness: u32,
    /// A palette role name or a CSS colour, passed through untouched.
    pub frame_color: String,
    /// Whether the corners should be listening for the pointer.
    pub hot_corners_active: bool,
}

impl ScreenChrome {
    /// What to draw, given the config and whether anything is full-screen.
    pub fn resolve(config: &crate::Config, fullscreen: bool) -> Self {
        let rounding = FakeRounding::from_config(config.appearance.fake_screen_rounding);

        Self {
            corners_visible: rounding.shows(fullscreen),
            radius: config.appearance.screen_rounding,
            frame_edges: frame_edges(&config.bar),
            frame_thickness: config.bar.frame_thickness,
            frame_color: config.bar.frame_color.clone(),
            // A hot corner firing during a full-screen game is the exact
            // annoyance the rounding setting already exists to avoid, so the
            // two go quiet together.
            hot_corners_active: config.sidebar.corner_open.enable && !fullscreen,
        }
    }
}

/// One of the four corners of the screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Corner {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

impl Corner {
    pub const ALL: [Self; 4] = [
        Self::TopLeft,
        Self::TopRight,
        Self::BottomLeft,
        Self::BottomRight,
    ];

    pub fn is_top(self) -> bool {
        matches!(self, Self::TopLeft | Self::TopRight)
    }

    pub fn is_left(self) -> bool {
        matches!(self, Self::TopLeft | Self::BottomLeft)
    }
}

/// A corner that does something, and the invisible patch that does it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct HotCorner {
    pub corner: Corner,
    /// Physical screen pixels, which is what a window region is measured in.
    pub rect: Rect,
    /// The `GlobalStates` flag this corner flips.
    pub action: String,
}

/// The invisible patches that open something, for the corners that have one.
///
/// The region is a wide, thin strip pressed into the corner rather than a
/// square — that is what makes it reachable by throwing the pointer at the
/// edge, which is the whole point of a hot corner.
///
/// Two rules matter, and both exist because a mis-placed region is invisible:
///
/// * A strip is **anchored into its own corner**, so a right-hand one starts
///   at `screen_width - width` rather than at zero.
/// * Opposite strips are **never allowed to meet**. On a narrow screen two
///   250-pixel strips would overlap in the middle, and a click aimed at one
///   corner's action would land on the other's.
pub fn hot_corners(config: &CornerOpen, screen: (i32, i32)) -> Vec<HotCorner> {
    if !config.enable {
        return Vec::new();
    }

    let (screen_width, screen_height) = screen;
    if screen_width <= 0 || screen_height <= 0 {
        return Vec::new();
    }

    // Half the screen each, at most, so the two strips on an edge cannot touch.
    let width = (config.corner_region_width as i32)
        .clamp(0, screen_width / 2)
        .min(screen_width);
    let height = (config.corner_region_height as i32)
        .clamp(0, screen_height / 2)
        .min(screen_height);
    if width == 0 || height == 0 {
        return Vec::new();
    }

    Corner::ALL
        .into_iter()
        .filter(|corner| config.bottom || corner.is_top())
        .filter_map(|corner| {
            let action = action_for(config, corner);
            // A corner bound to nothing is not a corner that does nothing
            // quietly — it is simply not a hot corner, and gets no region.
            if action.is_empty() {
                return None;
            }
            Some(HotCorner {
                corner,
                rect: Rect {
                    x: if corner.is_left() {
                        0
                    } else {
                        screen_width - width
                    },
                    y: if corner.is_top() {
                        0
                    } else {
                        screen_height - height
                    },
                    width,
                    height,
                },
                action,
            })
        })
        .collect()
}

fn action_for(config: &CornerOpen, corner: Corner) -> String {
    match corner {
        Corner::TopLeft => config.top_left_action.trim().to_owned(),
        Corner::TopRight => config.top_right_action.trim().to_owned(),
        Corner::BottomLeft => config.bottom_left_action.trim().to_owned(),
        Corner::BottomRight => config.bottom_right_action.trim().to_owned(),
    }
}

/// Which way a scroll on a corner is meant to be read.
///
/// Left corners are brightness and right corners are volume, which is
/// arbitrary but is what the original does — and being arbitrary is exactly
/// why it should not be decided twice.
pub fn scroll_target(corner: Corner) -> ScrollTarget {
    if corner.is_left() {
        ScrollTarget::Brightness
    } else {
        ScrollTarget::Volume
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ScrollTarget {
    Brightness,
    Volume,
}

/// One side of the screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Edge {
    Top,
    Bottom,
    Left,
    Right,
}

impl Edge {
    pub const ALL: [Self; 4] = [Self::Top, Self::Bottom, Self::Left, Self::Right];
}

/// Which edge the bar is against.
pub fn bar_edge(bar: &Bar) -> Edge {
    match (bar.vertical, bar.bottom) {
        (false, false) => Edge::Top,
        (false, true) => Edge::Bottom,
        (true, false) => Edge::Left,
        (true, true) => Edge::Right,
    }
}

/// The edges the frame is drawn on.
///
/// All four, except in one case: a bar in the hugging style already reaches
/// both ends of its edge, so a frame there would be a second line drawn on top
/// of the first. Unless the bar's content is only in the middle — then its ends
/// are empty and the frame has somewhere to go.
pub fn frame_edges(bar: &Bar) -> Vec<Edge> {
    if !bar.show_frame {
        return Vec::new();
    }

    let hugging = bar.enable && bar.style == "hug";
    let centre_only = bar.left.is_empty() && bar.right.is_empty();
    let bar_edge = bar_edge(bar);

    Edge::ALL
        .into_iter()
        .filter(|edge| !hugging || *edge != bar_edge || centre_only)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Config;

    const SCREEN: (i32, i32) = (1920, 1080);

    fn corners() -> CornerOpen {
        CornerOpen::default()
    }

    #[test]
    fn the_rounding_policy_reads_the_originals_numbers() {
        assert_eq!(FakeRounding::from_config(0), FakeRounding::Never);
        assert_eq!(FakeRounding::from_config(1), FakeRounding::Always);
        assert_eq!(
            FakeRounding::from_config(2),
            FakeRounding::WhenNotFullscreen
        );
        // A typo should not silently switch a feature off.
        assert_eq!(
            FakeRounding::from_config(99),
            FakeRounding::WhenNotFullscreen
        );
    }

    /// Rounded corners over a full-screen video are four notches cut out of
    /// the picture, which is the whole reason for the middle setting.
    #[test]
    fn the_corners_get_out_of_the_way_of_a_full_screen_window() {
        assert!(FakeRounding::WhenNotFullscreen.shows(false));
        assert!(!FakeRounding::WhenNotFullscreen.shows(true));
        assert!(FakeRounding::Always.shows(true));
        assert!(!FakeRounding::Never.shows(false));
    }

    #[test]
    fn each_strip_is_anchored_into_its_own_corner() {
        let found = hot_corners(&corners(), SCREEN);
        let by = |corner: Corner| {
            found
                .iter()
                .find(|hot| hot.corner == corner)
                .unwrap_or_else(|| panic!("{corner:?} is missing"))
                .rect
        };

        let width = by(Corner::TopLeft).width;
        assert_eq!(by(Corner::TopLeft).x, 0);
        assert_eq!(by(Corner::TopLeft).y, 0);
        // The trap: a right-hand strip drawn at x = 0 looks fine in a
        // screenshot of the left half of the screen.
        assert_eq!(by(Corner::TopRight).x, SCREEN.0 - width);
        assert_eq!(by(Corner::TopRight).y, 0);
    }

    /// Every strip has to be on the screen, whatever the config asked for.
    #[test]
    fn the_strips_stay_on_the_screen() {
        let mut config = corners();
        config.bottom = true;
        config.corner_region_width = 4000;
        config.corner_region_height = 4000;

        for hot in hot_corners(&config, SCREEN) {
            assert!(hot.rect.x >= 0, "{hot:?}");
            assert!(hot.rect.y >= 0, "{hot:?}");
            assert!(hot.rect.x + hot.rect.width <= SCREEN.0, "{hot:?}");
            assert!(hot.rect.y + hot.rect.height <= SCREEN.1, "{hot:?}");
        }
    }

    /// Two strips that met in the middle would make one corner's action fire
    /// on a click aimed at the other's.
    #[test]
    fn opposite_strips_never_touch() {
        let mut config = corners();
        config.bottom = true;
        // Wider than half of a narrow screen.
        config.corner_region_width = 250;
        let narrow = (400, 300);

        let found = hot_corners(&config, narrow);
        let left = found
            .iter()
            .find(|hot| hot.corner == Corner::TopLeft)
            .expect("a top left");
        let right = found
            .iter()
            .find(|hot| hot.corner == Corner::TopRight)
            .expect("a top right");

        assert!(
            left.rect.x + left.rect.width <= right.rect.x,
            "the strips overlap: {left:?} and {right:?}"
        );
    }

    #[test]
    fn the_bottom_corners_are_off_until_they_are_asked_for() {
        let found = hot_corners(&corners(), SCREEN);
        assert_eq!(found.len(), 2);
        assert!(found.iter().all(|hot| hot.corner.is_top()));

        let mut config = corners();
        config.bottom = true;
        assert_eq!(hot_corners(&config, SCREEN).len(), 4);
    }

    #[test]
    fn switching_the_corners_off_leaves_no_regions_at_all() {
        let mut config = corners();
        config.enable = false;
        assert!(hot_corners(&config, SCREEN).is_empty());
    }

    /// A corner bound to nothing gets no region, rather than an invisible
    /// patch of screen that swallows clicks and does nothing with them.
    #[test]
    fn a_corner_with_no_action_gets_no_region() {
        let mut config = corners();
        config.top_left_action = String::new();
        let found = hot_corners(&config, SCREEN);
        assert!(found.iter().all(|hot| hot.corner != Corner::TopLeft));
        assert_eq!(found.len(), 1);
    }

    #[test]
    fn a_screen_with_no_size_produces_no_regions() {
        assert!(hot_corners(&corners(), (0, 0)).is_empty());
        assert!(hot_corners(&corners(), (-1, -1)).is_empty());
    }

    #[test]
    fn scrolling_a_left_corner_is_brightness_and_a_right_one_is_volume() {
        assert_eq!(scroll_target(Corner::TopLeft), ScrollTarget::Brightness);
        assert_eq!(scroll_target(Corner::BottomLeft), ScrollTarget::Brightness);
        assert_eq!(scroll_target(Corner::TopRight), ScrollTarget::Volume);
        assert_eq!(scroll_target(Corner::BottomRight), ScrollTarget::Volume);
    }

    #[test]
    fn the_resolved_chrome_carries_the_rounding_decision() {
        let mut config = Config::default();
        config.appearance.fake_screen_rounding = 2;

        assert!(ScreenChrome::resolve(&config, false).corners_visible);
        assert!(!ScreenChrome::resolve(&config, true).corners_visible);

        config.appearance.fake_screen_rounding = 1;
        assert!(ScreenChrome::resolve(&config, true).corners_visible);
    }

    /// A hot corner firing during a full-screen game is the same annoyance the
    /// rounding setting exists to avoid, so the two go quiet together.
    #[test]
    fn the_hot_corners_go_quiet_with_the_rounding() {
        let config = Config::default();
        assert!(ScreenChrome::resolve(&config, false).hot_corners_active);
        assert!(!ScreenChrome::resolve(&config, true).hot_corners_active);
    }

    #[test]
    fn switching_the_corners_off_stops_them_listening_even_when_nothing_is_full_screen() {
        let mut config = Config::default();
        config.sidebar.corner_open.enable = false;
        assert!(!ScreenChrome::resolve(&config, false).hot_corners_active);
    }

    #[test]
    fn the_frame_is_off_until_it_is_asked_for() {
        assert!(frame_edges(&Config::default().bar).is_empty());
    }

    #[test]
    fn the_frame_takes_every_edge_when_the_bar_does_not_hug_one() {
        let mut config = Config::default();
        config.bar.show_frame = true;
        config.bar.style = "float".to_owned();
        assert_eq!(frame_edges(&config.bar).len(), 4);
    }

    /// A hugging bar already draws a line along its whole edge; a frame there
    /// would be a second line on top of the first.
    #[test]
    fn a_hugging_bar_keeps_the_frame_off_its_own_edge() {
        let mut config = Config::default();
        config.bar.show_frame = true;
        config.bar.style = "hug".to_owned();

        let edges = frame_edges(&config.bar);
        assert!(!edges.contains(&Edge::Top), "the bar is at the top");
        assert_eq!(edges.len(), 3);

        config.bar.bottom = true;
        assert!(!frame_edges(&config.bar).contains(&Edge::Bottom));

        config.bar.vertical = true;
        config.bar.bottom = false;
        assert!(!frame_edges(&config.bar).contains(&Edge::Left));
    }

    /// Unless the bar's ends are empty, in which case there is room for it.
    #[test]
    fn a_centre_only_bar_leaves_room_for_the_frame() {
        let mut config = Config::default();
        config.bar.show_frame = true;
        config.bar.style = "hug".to_owned();
        config.bar.left = Vec::new();
        config.bar.right = Vec::new();

        assert_eq!(frame_edges(&config.bar).len(), 4);
    }

    #[test]
    fn a_bar_that_is_switched_off_never_hugs_anything() {
        let mut config = Config::default();
        config.bar.show_frame = true;
        config.bar.style = "hug".to_owned();
        config.bar.enable = false;

        assert_eq!(frame_edges(&config.bar).len(), 4);
    }
}
