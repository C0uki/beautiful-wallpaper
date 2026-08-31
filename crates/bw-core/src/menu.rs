//! What the desktop menu offers, and where it goes.
//!
//! Two decisions, both easy to get wrong in ways nobody notices until the menu
//! is on screen. Which entries appear depends on the config *and* on what this
//! build can actually do — an entry that opens a surface which does not exist
//! is a line that does nothing. And where the menu goes depends on where the
//! pointer is: opened at the cursor without thinking, a menu near the right or
//! bottom edge hangs off the screen, and the entries that fell off are exactly
//! the ones at the bottom of the list.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::Config;

/// One line of the menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum MenuItem {
    /// Open the wallpaper browser.
    ChangeWallpaper,
    /// Another wallpaper from the same folder.
    NextWallpaper,
    /// Rearrange the desktop widgets.
    EditWidgets,
    /// The search overlay.
    Overview,
    /// Pick a region and save it.
    Screenshot,
    /// The way out of the session.
    Session,
    /// Windows' own display settings.
    DisplaySettings,
    /// Windows' own personalisation settings.
    Personalise,
}

impl MenuItem {
    /// Every entry, in the order the menu draws them.
    ///
    /// The shell's own entries come first and Windows' settings pages last,
    /// with a rule drawn between them by the frontend: they go somewhere
    /// entirely different when picked, and a menu that mixes them without
    /// saying so is a menu that surprises people.
    pub const ORDER: [Self; 8] = [
        Self::ChangeWallpaper,
        Self::NextWallpaper,
        Self::EditWidgets,
        Self::Overview,
        Self::Screenshot,
        Self::Session,
        Self::DisplaySettings,
        Self::Personalise,
    ];

    pub fn symbol(self) -> &'static str {
        match self {
            Self::ChangeWallpaper => "wallpaper",
            Self::NextWallpaper => "shuffle",
            Self::EditWidgets => "widgets",
            Self::Overview => "search",
            Self::Screenshot => "photo_camera",
            Self::Session => "power_settings_new",
            Self::DisplaySettings => "desktop_windows",
            Self::Personalise => "palette",
        }
    }

    /// Whether picking this leaves the shell for Windows' own interface.
    pub fn leaves_the_shell(self) -> bool {
        matches!(self, Self::DisplaySettings | Self::Personalise)
    }
}

/// The entries to draw, in the order to draw them.
///
/// Each entry has to clear two bars: the user left it switched on under
/// `desktopMenu`, *and* the thing it opens is switched on too. A menu line
/// pointing at a surface that has been turned off is a line that does nothing,
/// and the user has no way to tell that from a bug.
///
/// The two Windows settings pages clear the second bar by definition — they
/// are not this shell's to switch off.
pub fn items(config: &Config) -> Vec<MenuItem> {
    let menu = &config.desktop_menu;

    MenuItem::ORDER
        .into_iter()
        .filter(|item| match item {
            MenuItem::ChangeWallpaper => menu.change_wallpaper,
            MenuItem::NextWallpaper => menu.next_wallpaper,
            MenuItem::EditWidgets => menu.edit_widgets && config.background.widgets.enable,
            MenuItem::Overview => menu.overview && config.overview.enable,
            MenuItem::Screenshot => menu.screenshot && config.capture.enable,
            MenuItem::Session => menu.session && config.session.enable,
            MenuItem::DisplaySettings => menu.display_settings,
            MenuItem::Personalise => menu.personalise,
        })
        .collect()
}

/// A position, in whatever units the caller is working in.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Placement {
    pub x: i32,
    pub y: i32,
}

/// Where to put a menu of this size, opened at this point.
///
/// The menu opens down and to the right of the pointer, which is what every
/// menu on the platform does — until there is no room, at which point it
/// **flips to the other side of the cursor** rather than being nudged back
/// onto the screen. Nudging leaves the pointer sitting on top of an entry the
/// user did not aim at, and one twitch selects it.
///
/// A menu too large for the screen in either direction is pinned to the top
/// left corner: it will be clipped whatever happens, and clipping the end of
/// the list is better than clipping its beginning.
pub fn place(at: Placement, menu: (i32, i32), screen: (i32, i32), margin: i32) -> Placement {
    let (width, height) = menu;
    let (screen_width, screen_height) = screen;

    let x = if at.x + width + margin <= screen_width {
        at.x
    } else if at.x - width >= margin {
        at.x - width
    } else {
        margin.min((screen_width - width).max(0))
    };

    let y = if at.y + height + margin <= screen_height {
        at.y
    } else if at.y - height >= margin {
        at.y - height
    } else {
        margin.min((screen_height - height).max(0))
    };

    Placement { x, y }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCREEN: (i32, i32) = (1920, 1080);
    const MENU: (i32, i32) = (240, 320);
    const MARGIN: i32 = 8;

    fn at(x: i32, y: i32) -> Placement {
        Placement { x, y }
    }

    fn placed(x: i32, y: i32) -> Placement {
        place(at(x, y), MENU, SCREEN, MARGIN)
    }

    #[test]
    fn there_is_room_so_the_menu_opens_where_it_was_asked_to() {
        assert_eq!(placed(100, 100), at(100, 100));
    }

    /// Nudged back on screen, the pointer ends up on an entry nobody aimed at.
    #[test]
    fn near_the_right_edge_the_menu_flips_rather_than_sliding() {
        let found = placed(1800, 100);
        assert_eq!(found.x, 1800 - MENU.0, "flipped to the left of the cursor");
        assert_eq!(found.y, 100);
    }

    #[test]
    fn near_the_bottom_edge_it_flips_upwards() {
        let found = placed(100, 900);
        assert_eq!(found.x, 100);
        assert_eq!(found.y, 900 - MENU.1);
    }

    #[test]
    fn in_the_bottom_right_corner_it_flips_both_ways() {
        assert_eq!(placed(1800, 900), at(1800 - MENU.0, 900 - MENU.1));
    }

    /// Whichever way it is placed, it has to be on the screen.
    #[test]
    fn the_menu_always_ends_up_on_screen() {
        for x in (0..=1920).step_by(97) {
            for y in (0..=1080).step_by(53) {
                let found = placed(x, y);
                assert!(found.x >= 0, "off the left at {x},{y}");
                assert!(found.y >= 0, "off the top at {x},{y}");
                assert!(
                    found.x + MENU.0 <= SCREEN.0,
                    "off the right at {x},{y}: {found:?}"
                );
                assert!(
                    found.y + MENU.1 <= SCREEN.1,
                    "off the bottom at {x},{y}: {found:?}"
                );
            }
        }
    }

    /// A menu that cannot fit is clipped at its end rather than its beginning.
    #[test]
    fn a_menu_taller_than_the_screen_starts_at_the_top() {
        let tall = place(at(100, 500), (240, 2000), SCREEN, MARGIN);
        assert_eq!(tall.y, 0);

        let wide = place(at(500, 100), (4000, 320), SCREEN, MARGIN);
        assert_eq!(wide.x, 0);
    }

    #[test]
    fn a_click_in_the_very_corner_still_places_the_menu() {
        let found = placed(1920, 1080);
        assert!(found.x >= 0 && found.y >= 0);
        assert!(found.x + MENU.0 <= SCREEN.0 && found.y + MENU.1 <= SCREEN.1);
    }

    #[test]
    fn the_entries_come_out_in_a_fixed_order() {
        assert_eq!(items(&Config::default()), MenuItem::ORDER.to_vec());
    }

    #[test]
    fn switching_one_off_removes_it() {
        let mut config = Config::default();
        config.desktop_menu.next_wallpaper = false;
        config.desktop_menu.personalise = false;

        let found = items(&config);
        assert!(!found.contains(&MenuItem::NextWallpaper));
        assert!(!found.contains(&MenuItem::Personalise));
        assert!(found.contains(&MenuItem::ChangeWallpaper));
    }

    /// The menu entry survives its own checkbox but not the absence of the
    /// thing it opens.
    #[test]
    fn an_entry_whose_surface_is_switched_off_does_not_appear() {
        let mut config = Config::default();
        config.overview.enable = false;
        config.session.enable = false;
        config.capture.enable = false;
        config.background.widgets.enable = false;

        let found = items(&config);
        for absent in [
            MenuItem::Overview,
            MenuItem::Session,
            MenuItem::Screenshot,
            MenuItem::EditWidgets,
        ] {
            assert!(!found.contains(&absent), "{absent:?} opens nothing");
        }
        // Windows' own pages are not this shell's to switch off.
        assert!(found.contains(&MenuItem::DisplaySettings));
        assert!(found.contains(&MenuItem::Personalise));
    }

    /// The two that hand the user over to Windows are told apart, so the menu
    /// can put a rule in front of them.
    #[test]
    fn the_entries_that_leave_the_shell_are_the_last_two() {
        let leaving: Vec<MenuItem> = MenuItem::ORDER
            .into_iter()
            .filter(|item| item.leaves_the_shell())
            .collect();
        assert_eq!(
            leaving,
            vec![MenuItem::DisplaySettings, MenuItem::Personalise]
        );

        let first_leaver = MenuItem::ORDER
            .iter()
            .position(|item| item.leaves_the_shell())
            .expect("one of them leaves");
        assert!(
            MenuItem::ORDER[first_leaver..]
                .iter()
                .all(|item| item.leaves_the_shell()),
            "they have to be contiguous at the end for one rule to separate them"
        );
    }
}
