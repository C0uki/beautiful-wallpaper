//! The floating overlay: what is on it, where, and which window each part
//! belongs to.
//!
//! The overlay is a canvas of small widgets the user drags around. Most of it
//! is ordinary, and then there is pinning, which is the whole point: a pinned
//! widget **stays on screen after the overlay closes**. A crosshair is only
//! useful while you are playing, which is exactly when the overlay is shut.
//!
//! That is what makes this need two windows rather than one. Windows has no
//! separate input region — `SetWindowRgn` decides what is drawn *and* what is
//! clickable together — so a pinned crosshair, which must be visible and must
//! not take the pointer, cannot live in the same window as a pinned note,
//! which must be visible and must. Splitting them by that one property is what
//! this module does, and getting it wrong means either a crosshair that eats
//! every click in the middle of the screen or a note nobody can type into.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::capture::Rect;
use crate::config::Overlay;
use crate::persistent::{OverlayState, OverlayWidgetState};

/// The widgets the overlay can put on screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum OverlayWidget {
    Crosshair,
    Notes,
    Resources,
}

impl OverlayWidget {
    pub const ALL: [Self; 3] = [Self::Crosshair, Self::Notes, Self::Resources];

    /// The name used in the config, the CLI and the persisted state.
    pub fn keyword(self) -> &'static str {
        match self {
            Self::Crosshair => "crosshair",
            Self::Notes => "notes",
            Self::Resources => "resources",
        }
    }

    pub fn from_keyword(keyword: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|widget| widget.keyword() == keyword)
    }

    pub fn symbol(self) -> &'static str {
        match self {
            Self::Crosshair => "point_scan",
            Self::Notes => "note_stack",
            Self::Resources => "browse_activity",
        }
    }

    /// Whether this widget makes sense with the pointer passing through it.
    ///
    /// The crosshair does and the other two do not: a crosshair is something
    /// to aim with, and one that swallowed clicks in the middle of the screen
    /// would be unusable the moment it was pinned.
    pub fn default_clickthrough(self) -> bool {
        matches!(self, Self::Crosshair)
    }

    /// The size it starts at, before the user drags it anywhere.
    pub fn default_size(self) -> (i32, i32) {
        match self {
            // Set from the crosshair code rather than from here; this is only
            // what the box is until the code has been read.
            Self::Crosshair => (64, 64),
            Self::Notes => (280, 220),
            Self::Resources => (320, 210),
        }
    }
}

/// One widget, placed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Placed {
    pub widget: OverlayWidget,
    pub rect: Rect,
    pub pinned: bool,
    pub clickthrough: bool,
}

/// Where every part of the overlay goes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct OverlayLayout {
    /// Drawn in the window that takes the pointer.
    pub interactive: Vec<Placed>,
    /// Drawn in the window that never does.
    pub passive: Vec<Placed>,
    /// The interactive window's input region.
    ///
    /// `None` means the whole screen — the overlay is open, and every click
    /// belongs to it, including the one on the backdrop that closes it. A list
    /// means the overlay is shut and only these rectangles are still live.
    pub region: Option<Vec<Rect>>,
    /// Whether the interactive window should be on screen at all.
    pub interactive_visible: bool,
    pub passive_visible: bool,
    /// Whether to darken what is behind. Only ever while the overlay is open.
    pub scrim: bool,
}

/// Keeps a rectangle inside the screen.
///
/// A widget dragged past the edge and left there is a widget the user cannot
/// get back — the overlay draws it where it was told, and off-screen is a
/// place it can be told about.
pub fn clamp(rect: Rect, screen: (i32, i32)) -> Rect {
    let (screen_width, screen_height) = screen;
    // A widget wider than the screen is pinned to the left rather than pushed
    // off it: clipping the far edge is better than clipping the near one.
    let width = rect.width.clamp(1, screen_width.max(1));
    let height = rect.height.clamp(1, screen_height.max(1));

    Rect {
        x: rect.x.clamp(0, (screen_width - width).max(0)),
        y: rect.y.clamp(0, (screen_height - height).max(0)),
        width,
        height,
    }
}

/// Works out what each of the overlay's two windows should be showing.
pub fn layout(
    state: &OverlayState,
    config: &Overlay,
    screen: (i32, i32),
    open: bool,
) -> OverlayLayout {
    let placed: Vec<Placed> = state
        .open
        .iter()
        .filter_map(|keyword| OverlayWidget::from_keyword(keyword))
        // The same widget listed twice would be drawn twice on top of itself,
        // and the second one would take every click meant for the first.
        .fold(Vec::new(), |mut found: Vec<OverlayWidget>, widget| {
            if !found.contains(&widget) {
                found.push(widget);
            }
            found
        })
        .into_iter()
        .map(|widget| place(widget, widget_state(state, widget), screen))
        .collect();

    if !config.enable {
        return OverlayLayout {
            interactive: Vec::new(),
            passive: Vec::new(),
            region: Some(Vec::new()),
            interactive_visible: false,
            passive_visible: false,
            scrim: false,
        };
    }

    // While the overlay is open everything is interactive, including the
    // crosshair: positioning it is what the open overlay is for.
    if open {
        return OverlayLayout {
            interactive: placed,
            passive: Vec::new(),
            region: None,
            interactive_visible: true,
            passive_visible: false,
            scrim: config.darken_screen,
        };
    }

    // Shut: only what was pinned survives, split by whether it takes clicks.
    let (passive, interactive): (Vec<Placed>, Vec<Placed>) = placed
        .into_iter()
        .filter(|found| found.pinned)
        .partition(|found| found.clickthrough);

    let region: Vec<Rect> = interactive.iter().map(|found| found.rect).collect();

    OverlayLayout {
        interactive_visible: !interactive.is_empty(),
        passive_visible: !passive.is_empty(),
        region: Some(region),
        interactive,
        passive,
        scrim: false,
    }
}

fn widget_state(state: &OverlayState, widget: OverlayWidget) -> &OverlayWidgetState {
    match widget {
        OverlayWidget::Crosshair => &state.crosshair,
        OverlayWidget::Notes => &state.notes,
        OverlayWidget::Resources => &state.resources,
    }
}

fn place(widget: OverlayWidget, state: &OverlayWidgetState, screen: (i32, i32)) -> Placed {
    let (default_width, default_height) = widget.default_size();
    let rect = Rect {
        x: state.x,
        y: state.y,
        width: if state.width > 0 {
            state.width
        } else {
            default_width
        },
        height: if state.height > 0 {
            state.height
        } else {
            default_height
        },
    };

    Placed {
        widget,
        rect: clamp(rect, screen),
        pinned: state.pinned,
        clickthrough: state.clickthrough,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCREEN: (i32, i32) = (1920, 1080);

    fn state() -> OverlayState {
        OverlayState::default()
    }

    fn config() -> Overlay {
        Overlay::default()
    }

    fn open_with(keywords: &[&str]) -> OverlayState {
        let mut found = state();
        found.open = keywords.iter().map(|word| (*word).to_owned()).collect();
        found
    }

    #[test]
    fn every_widget_has_a_keyword_that_maps_back() {
        for widget in OverlayWidget::ALL {
            assert_eq!(OverlayWidget::from_keyword(widget.keyword()), Some(widget));
        }
        assert_eq!(OverlayWidget::from_keyword("nope"), None);
    }

    /// Only the crosshair is something to aim with; the other two are things
    /// to press.
    #[test]
    fn only_the_crosshair_passes_the_pointer_through_by_default() {
        assert!(OverlayWidget::Crosshair.default_clickthrough());
        assert!(!OverlayWidget::Notes.default_clickthrough());
        assert!(!OverlayWidget::Resources.default_clickthrough());
    }

    /// A widget dragged past the edge and left there is one the user cannot
    /// get back.
    #[test]
    fn a_widget_dragged_off_the_screen_comes_back() {
        let off = Rect {
            x: 5000,
            y: -400,
            width: 200,
            height: 100,
        };
        let found = clamp(off, SCREEN);
        assert!(found.x >= 0 && found.y >= 0);
        assert_eq!(found.x + found.width, SCREEN.0);
        assert_eq!(found.y, 0);
    }

    #[test]
    fn a_widget_larger_than_the_screen_is_pinned_to_the_corner() {
        let huge = Rect {
            x: 100,
            y: 100,
            width: 4000,
            height: 4000,
        };
        let found = clamp(huge, SCREEN);
        assert_eq!((found.x, found.y), (0, 0));
        assert_eq!((found.width, found.height), SCREEN);
    }

    #[test]
    fn an_open_overlay_makes_everything_interactive() {
        let found = layout(&open_with(&["crosshair", "notes"]), &config(), SCREEN, true);

        assert_eq!(found.interactive.len(), 2, "including the crosshair");
        assert!(found.passive.is_empty());
        assert!(
            found.region.is_none(),
            "an open overlay owns the whole screen, backdrop included"
        );
        assert!(found.interactive_visible);
        assert!(found.scrim);
    }

    /// The whole point of pinning: closing the overlay leaves the pinned ones
    /// behind and takes everything else away.
    #[test]
    fn closing_keeps_only_what_was_pinned() {
        let mut state = open_with(&["crosshair", "notes", "resources"]);
        state.crosshair.pinned = true;
        state.notes.pinned = true;
        // resources stays unpinned

        let found = layout(&state, &config(), SCREEN, false);
        let kept: Vec<OverlayWidget> = found
            .interactive
            .iter()
            .chain(found.passive.iter())
            .map(|placed| placed.widget)
            .collect();

        assert!(kept.contains(&OverlayWidget::Crosshair));
        assert!(kept.contains(&OverlayWidget::Notes));
        assert!(!kept.contains(&OverlayWidget::Resources));
        assert!(!found.scrim, "a shut overlay never darkens anything");
    }

    /// The split that the two windows exist for. A pinned crosshair in the
    /// interactive window would swallow every click in the middle of the
    /// screen; a pinned note in the passive one could never be typed into.
    #[test]
    fn a_pinned_crosshair_and_a_pinned_note_go_to_different_windows() {
        let mut state = open_with(&["crosshair", "notes"]);
        state.crosshair.pinned = true;
        state.crosshair.clickthrough = true;
        state.notes.pinned = true;
        state.notes.clickthrough = false;

        let found = layout(&state, &config(), SCREEN, false);

        assert_eq!(found.passive.len(), 1);
        assert_eq!(found.passive[0].widget, OverlayWidget::Crosshair);
        assert_eq!(found.interactive.len(), 1);
        assert_eq!(found.interactive[0].widget, OverlayWidget::Notes);
        assert!(found.interactive_visible && found.passive_visible);
    }

    /// The region is what stops the shut overlay's window covering the
    /// desktop. An empty one with the window still shown would swallow
    /// everything.
    #[test]
    fn a_shut_overlay_cuts_its_window_down_to_the_pinned_widgets() {
        let mut state = open_with(&["notes"]);
        state.notes.pinned = true;
        state.notes.clickthrough = false;
        state.notes.x = 40;
        state.notes.y = 60;
        state.notes.width = 300;
        state.notes.height = 200;

        let found = layout(&state, &config(), SCREEN, false);
        assert_eq!(
            found.region,
            Some(vec![Rect {
                x: 40,
                y: 60,
                width: 300,
                height: 200
            }])
        );
    }

    #[test]
    fn nothing_pinned_means_neither_window_is_shown() {
        let found = layout(
            &open_with(&["crosshair", "notes"]),
            &config(),
            SCREEN,
            false,
        );
        assert!(!found.interactive_visible);
        assert!(!found.passive_visible);
        assert_eq!(found.region, Some(Vec::new()));
    }

    #[test]
    fn switching_the_overlay_off_leaves_nothing_on_screen() {
        let mut config = config();
        config.enable = false;
        let mut state = open_with(&["crosshair"]);
        state.crosshair.pinned = true;

        let found = layout(&state, &config, SCREEN, true);
        assert!(!found.interactive_visible && !found.passive_visible);
        assert!(found.interactive.is_empty() && found.passive.is_empty());
    }

    /// A hand-edited state file can name the same widget twice; drawn twice,
    /// the copy on top takes every click meant for the one underneath.
    #[test]
    fn the_same_widget_listed_twice_is_placed_once() {
        let found = layout(
            &open_with(&["notes", "notes", "notes"]),
            &config(),
            SCREEN,
            true,
        );
        assert_eq!(found.interactive.len(), 1);
    }

    #[test]
    fn a_widget_nobody_has_heard_of_is_skipped_rather_than_drawn_blank() {
        let found = layout(&open_with(&["notes", "teapot"]), &config(), SCREEN, true);
        assert_eq!(found.interactive.len(), 1);
        assert_eq!(found.interactive[0].widget, OverlayWidget::Notes);
    }

    /// A state file with no size in it — the first run, or a hand-edited one —
    /// still has to produce a box somebody can see.
    #[test]
    fn a_widget_with_no_stored_size_gets_its_own_default() {
        let found = layout(&open_with(&["resources"]), &config(), SCREEN, true);
        let rect = found.interactive[0].rect;
        assert_eq!(
            (rect.width, rect.height),
            OverlayWidget::Resources.default_size()
        );
    }

    #[test]
    fn the_scrim_follows_the_config() {
        let mut config = config();
        config.darken_screen = false;
        let found = layout(&open_with(&["notes"]), &config, SCREEN, true);
        assert!(!found.scrim);
    }
}
