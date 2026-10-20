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
    if bar.vertical {
        Edge::Left
    } else {
        Edge::Top
    }
}

/// The edges the frame is drawn on: all four, but the bar's own. The bar
/// already runs the length of its edge, so a frame there would be a second
/// line drawn on top of the first.
pub fn frame_edges(bar: &Bar) -> Vec<Edge> {
    if !bar.show_frame {
        return Vec::new();
    }
    Edge::ALL
        .into_iter()
        .filter(|edge| !bar.enable || *edge != bar_edge(bar))
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
    fn the_frame_keeps_off_the_bar_s_own_edge() {
        let mut config = Config::default();
        config.bar.show_frame = true;

        let edges = frame_edges(&config.bar);
        assert!(!edges.contains(&Edge::Top), "the bar is at the top");
        assert_eq!(edges.len(), 3);

        config.bar.vertical = true;
        assert!(!frame_edges(&config.bar).contains(&Edge::Left));
    }

    #[test]
    fn with_the_bar_switched_off_the_frame_takes_every_edge() {
        let mut config = Config::default();
        config.bar.show_frame = true;
        config.bar.enable = false;

        assert_eq!(frame_edges(&config.bar).len(), 4);
    }
}
