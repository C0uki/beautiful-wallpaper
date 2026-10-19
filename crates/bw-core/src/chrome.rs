//! The screen's own chrome: a frame around the display.
//!
//! It lives at the very edge of the display, where a mistake in the arithmetic
//! — a line drawn twice, or on the wrong edge — has nothing else drawn near it
//! to make it obvious, so where it goes is decided here, under tests.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::config::Bar;

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
    /// The edges the frame is drawn on. Empty when there is no frame.
    pub frame_edges: Vec<Edge>,
    pub frame_thickness: u32,
    /// A palette role name or a CSS colour, passed through untouched.
    pub frame_color: String,
}

impl ScreenChrome {
    /// What to draw, given the config.
    pub fn resolve(config: &crate::Config) -> Self {
        Self {
            frame_edges: frame_edges(&config.bar),
            frame_thickness: config.bar.frame_thickness,
            frame_color: config.bar.frame_color.clone(),
        }
    }
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
