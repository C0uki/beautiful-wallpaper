//! The configuration schema.
//!
//! Mirrors the key names of end4-pC's `modules/common/Config.qml` so that a user
//! moving between the two has the same vocabulary. Keys that are meaningless on
//! Windows (`hyprland.*`) are replaced by [`WindowsIntegration`] under `windows`.
//!
//! As in the original, the schema *is* the set of defaults: every field carries a
//! value, `#[serde(default)]` fills in anything a hand-edited file leaves out, and
//! a missing file is materialised by serialising [`Config::default`].

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Declares a config struct with camelCase JSON keys, a `Default` built from the
/// per-field expressions, and TypeScript bindings.
macro_rules! config_struct {
    (
        $(#[$meta:meta])*
        pub struct $name:ident {
            $(
                $(#[doc = $doc:literal])*
                pub $field:ident : $ty:ty = $default:expr,
            )*
        }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
        #[serde(rename_all = "camelCase", default, deny_unknown_fields)]
        #[ts(export)]
        pub struct $name {
            $(
                $(#[doc = $doc])*
                pub $field: $ty,
            )*
        }

        impl Default for $name {
            fn default() -> Self {
                Self {
                    $( $field: $default, )*
                }
            }
        }
    };
}

fn s(value: &str) -> String {
    value.to_owned()
}

config_struct! {
    /// Root of `config.json`.
    pub struct Config {
        pub appearance: Appearance = Appearance::default(),
        pub audio: Audio = Audio::default(),
        pub background: Background = Background::default(),
        pub bar: Bar = Bar::default(),
        pub dock: Dock = Dock::default(),
        pub hacks: Hacks = Hacks::default(),
        pub language: Language = Language::default(),
        pub notifications: Notifications = Notifications::default(),
        pub osd: Osd = Osd::default(),
        pub policies: Policies = Policies::default(),
        pub resources: Resources = Resources::default(),
        pub sidebar: Sidebar = Sidebar::default(),
        pub time: Time = Time::default(),
        pub wallpaper_selector: WallpaperSelector = WallpaperSelector::default(),
        pub weather: Weather = Weather::default(),
        pub windows: WindowsIntegration = WindowsIntegration::default(),
        pub work_safety: WorkSafety = WorkSafety::default(),
    }
}

config_struct! {
    pub struct Appearance {
        /// `"auto"` picks a scheme variant per wallpaper, otherwise a fixed
        /// Material variant name (`tonalSpot`, `neutral`, `vibrant`, ...).
        pub palette: Palette = Palette::default(),
        pub fonts: Fonts = Fonts::default(),
        /// Extra translucency applied on top of the wallpaper-derived value.
        pub transparency: Transparency = Transparency::default(),
        pub wallpaper_theming: WallpaperTheming = WallpaperTheming::default(),
        /// Corner rounding multiplier applied to every surface.
        pub rounding_scale: f64 = 1.0,
    }
}

config_struct! {
    pub struct Palette {
        /// `"auto"` | `"tonalSpot"` | `"neutral"` | `"vibrant"` | `"expressive"`
        /// | `"content"` | `"fidelity"` | `"monochrome"` | `"rainbow"`
        /// | `"fruitSalad"`
        pub r#type: String = s("auto"),
        /// When set, overrides the colour extracted from the wallpaper.
        pub accent_color: Option<String> = None,
        /// `"auto"` follows the wallpaper's luminance, else `"light"` / `"dark"`.
        pub mode: String = s("dark"),
    }
}

config_struct! {
    pub struct Fonts {
        pub main: String = s("Segoe UI Variable Text, Segoe UI, sans-serif"),
        pub title: String = s("Segoe UI Variable Display, Segoe UI, sans-serif"),
        pub monospace: String = s("Cascadia Code, Consolas, monospace"),
        pub reading: String = s("Georgia, serif"),
        pub expressive: String = s("Segoe UI Variable Display, Segoe UI, sans-serif"),
        pub pixel_size: f64 = 15.0,
    }
}

config_struct! {
    pub struct Transparency {
        pub enable: bool = true,
        /// Added to the wallpaper-derived background transparency.
        pub extra: f64 = 0.0,
    }
}

config_struct! {
    pub struct WallpaperTheming {
        /// Recolour the OS accent colour and light/dark mode from the wallpaper.
        pub sync_system_accent: bool = true,
        /// Write a matching colour scheme into Windows Terminal's settings.
        pub sync_windows_terminal: bool = false,
    }
}

config_struct! {
    pub struct Background {
        pub wallpaper_path: String = String::new(),
        /// Extracted still frame, for video wallpapers.
        pub thumbnail_path: String = String::new(),
        /// Transition played when the wallpaper changes.
        pub wallpaper_animation: String = s("circle"),
        pub transition_duration: u32 = 1200,
        /// Render the wallpaper clipped into a Material shape, centred.
        pub centered_wallpaper: bool = false,
        pub centered_wallpaper_shape: String = s("clover"),
        pub centered_wallpaper_size: f64 = 0.55,
        pub parallax: Parallax = Parallax::default(),
        pub widgets: DesktopWidgets = DesktopWidgets::default(),
    }
}

config_struct! {
    pub struct Parallax {
        pub enable: bool = true,
        /// Zoom applied so panning never exposes an edge.
        pub zoom: f64 = 1.07,
        pub workspace_pan: f64 = 0.6,
    }
}

config_struct! {
    pub struct DesktopWidgets {
        pub enable: bool = true,
        /// Snap-to-grid step, in pixels, for dragged widgets.
        pub grid: u32 = 8,
        pub clock: WidgetPlacement = WidgetPlacement::at("clock", 0.04, 0.06),
        pub media: WidgetPlacement = WidgetPlacement::at("media", 0.04, 0.30),
        pub weather: WidgetPlacement = WidgetPlacement::at("weather", 0.72, 0.05),
        pub resources: WidgetPlacement = WidgetPlacement::at("resources", 0.72, 0.20),
        pub calendar: WidgetPlacement = WidgetPlacement::hidden("calendar", 0.72, 0.45),
        pub user_card: WidgetPlacement = WidgetPlacement::hidden("userCard", 0.72, 0.60),
        pub notes: WidgetPlacement = WidgetPlacement::hidden("notes", 0.04, 0.62),
    }
}

config_struct! {
    /// Position and visibility of one desktop widget.
    pub struct WidgetPlacement {
        pub id: String = String::new(),
        pub enable: bool = true,
        /// Fraction of the monitor's width, so placement survives resolution changes.
        pub x: f64 = 0.0,
        pub y: f64 = 0.0,
        /// `"free"` keeps the stored position; `"leastBusy"` moves the widget to
        /// the calmest region of the current wallpaper.
        pub placement_strategy: String = s("free"),
        pub style: String = s("default"),
    }
}

impl WidgetPlacement {
    fn at(id: &str, x: f64, y: f64) -> Self {
        Self {
            id: id.to_owned(),
            x,
            y,
            ..Self::default()
        }
    }

    fn hidden(id: &str, x: f64, y: f64) -> Self {
        Self {
            enable: false,
            ..Self::at(id, x, y)
        }
    }
}

config_struct! {
    pub struct Bar {
        pub enable: bool = true,
        /// Anchor the bar to the bottom edge instead of the top.
        pub bottom: bool = false,
        pub vertical: bool = false,
        pub height: u32 = 40,
        /// Reserve screen space through `SHAppBarMessage` so maximised windows
        /// keep clear of the bar.
        pub reserve_space: bool = true,
        pub auto_hide: bool = false,
        /// `"hug"` | `"float"` | `"islands"` | `"m3"`
        pub style: String = s("m3"),
        pub left: Vec<String> = vec![s("media")],
        pub center: Vec<String> = vec![s("workspaces"), s("activeWindow")],
        pub right: Vec<String> = vec![
            s("tray"),
            s("resources"),
            s("network"),
            s("battery"),
            s("utilButtons"),
            s("clock"),
        ],
    }
}

config_struct! {
    pub struct Dock {
        pub enable: bool = false,
        pub pinned_apps: Vec<String> = Vec::new(),
        pub auto_hide: bool = true,
        pub icon_size: u32 = 44,
    }
}

config_struct! {
    /// The transient readout shown when volume or brightness changes.
    pub struct Osd {
        pub enable: bool = true,
        /// Milliseconds the readout stays up after the last change.
        pub timeout: u32 = 1000,
        /// `"top"` or `"bottom"`. The readout clears the bar on whichever edge
        /// the bar occupies, so this is only about which end of the screen.
        pub position: String = s("top"),
        pub volume: bool = true,
        /// Brightness is unavailable on some displays; the readout is simply
        /// not shown when the platform cannot report a level.
        pub brightness: bool = true,
    }
}

config_struct! {
    pub struct Notifications {
        pub enable: bool = true,
        /// Milliseconds a toast stays up. Urgent notifications ignore this.
        pub timeout: u32 = 7000,
        /// One of `top_left`, `top_center`, `top_right`, `bottom_left`,
        /// `bottom_center`, `bottom_right`.
        pub position: String = s("top_right"),
        /// Toasts beyond this stay in the centre without ever popping up.
        pub max_visible: u32 = 4,
        /// Suppresses toasts without discarding the notifications themselves.
        pub do_not_disturb: bool = false,
        pub width: u32 = 380,
    }
}

config_struct! {
    pub struct Audio {
        /// Percentage points per volume step.
        pub step: u32 = 5,
        pub protection: HearingProtection = HearingProtection::default(),
    }
}

config_struct! {
    /// Guards against the volume jumping to something painful, which the
    /// original shell also does.
    pub struct HearingProtection {
        pub enable: bool = true,
        /// Volume is not allowed above this by the shell's own controls.
        pub max_volume: u32 = 100,
    }
}

config_struct! {
    /// The right sidebar: the shell's control centre.
    pub struct Sidebar {
        pub enable: bool = true,
        /// Fraction of the screen width the panel occupies.
        pub width: f64 = 0.26,
        /// Show the wallpaper banner with the avatar and uptime, rather than a
        /// plain row of system buttons.
        pub banner: bool = true,
        /// Overrides the banner image; empty means the current wallpaper.
        pub banner_image: String = String::new(),
        pub media_player: bool = true,
        pub notification_centre: bool = true,
        pub profile: Profile = Profile::default(),
        pub quick_toggles: QuickToggles = QuickToggles::default(),
        pub quick_sliders: QuickSliders = QuickSliders::default(),
        pub night_light: NightLight = NightLight::default(),
    }
}

config_struct! {
    pub struct Profile {
        /// Empty means the Windows account name.
        pub display_name: String = String::new(),
        /// Empty means the account picture Windows already has.
        pub avatar_path: String = String::new(),
    }
}

config_struct! {
    pub struct QuickToggles {
        pub enable: bool = true,
        /// `"classic"` for a single row of small buttons, `"android"` for the
        /// editable grid of tiles. Both are built; this only picks which.
        pub style: String = s("android"),
    }
}

config_struct! {
    pub struct QuickSliders {
        pub enable: bool = true,
        pub show_brightness: bool = true,
        pub show_volume: bool = true,
        pub show_mic: bool = true,
    }
}

config_struct! {
    /// A warm-tint overlay, applied by the shell through the display's gamma
    /// ramp rather than by driving Windows' own Night Light — that setting
    /// lives in an undocumented registry blob with no supported API.
    pub struct NightLight {
        pub enable: bool = false,
        /// Colour temperature in kelvin. 6500 is neutral; lower is warmer.
        pub temperature: u32 = 4000,
        /// Turn it on and off with the clock rather than by hand.
        pub automatic: bool = false,
        /// 24-hour local times, used only when `automatic` is set.
        pub from: String = s("20:00"),
        pub to: String = s("07:00"),
    }
}

config_struct! {
    pub struct Resources {
        /// Sampling interval for CPU/RAM/disk, in milliseconds.
        pub poll_interval: u32 = 2000,
        pub show_swap: bool = false,
    }
}

config_struct! {
    pub struct Time {
        pub format: String = s("HH:mm"),
        pub date_format: String = s("ddd, dd/MM"),
        pub week_starts_on_monday: bool = true,
    }
}

config_struct! {
    pub struct Language {
        /// `"auto"` follows the OS UI language.
        pub ui: String = s("auto"),
    }
}

config_struct! {
    pub struct Policies {
        /// 0 = off, 1 = on, 2 = local only.
        pub ai: u8 = 1,
        pub weeb: u8 = 0,
    }
}

config_struct! {
    pub struct WallpaperSelector {
        pub user_path: String = String::new(),
        pub columns: u32 = 4,
        pub show_searchbar: bool = true,
        pub close_after_selection: bool = true,
        /// Seconds between automatic wallpaper rotations; 0 disables it.
        pub change_interval: u32 = 0,
        pub extensions: Vec<String> = vec![
            s("jpg"), s("jpeg"), s("png"), s("webp"), s("bmp"), s("gif"),
        ],
        pub online: OnlineWallpapers = OnlineWallpapers::default(),
    }
}

config_struct! {
    pub struct OnlineWallpapers {
        pub enable: bool = true,
        /// `"wallhaven"` | `"unsplash"` | `"pexels"`
        pub default_provider: String = s("wallhaven"),
        /// `"1080p"` | `"2k"` | `"4k"`
        pub resolution: String = s("1080p"),
        /// Wallhaven purity filter: `"sfw"` | `"sketchy"` | `"nsfw"`.
        pub purity: String = s("sfw"),
        pub category: String = s("general"),
        /// Where downloads land; empty means `%USERPROFILE%\Pictures\Wallpapers`.
        pub download_path: String = String::new(),
    }
}

config_struct! {
    pub struct Weather {
        pub enable: bool = true,
        /// Empty means the location is resolved from the public IP.
        pub city: String = String::new(),
        pub use_usc_units: bool = false,
        pub refresh_interval: u32 = 900,
    }
}

config_struct! {
    /// The Windows counterpart of end4-pC's `hyprland` section.
    pub struct WindowsIntegration {
        /// `"auto"` probes for GlazeWM then komorebi; `"none"` disables
        /// workspace integration entirely.
        pub window_manager: String = s("auto"),
        pub glazewm: GlazeWm = GlazeWm::default(),
        pub komorebi: Komorebi = Komorebi::default(),
        /// Hide the stock Windows taskbar while the shell's own bar is running.
        pub hide_system_taskbar: bool = false,
        pub start_with_windows: bool = false,
        /// Blur behind panels: `"auto"` picks Mica on Windows 11 and Acrylic on
        /// Windows 10; `"acrylic"`, `"mica"` and `"none"` force one.
        pub backdrop: String = s("auto"),
    }
}

config_struct! {
    pub struct GlazeWm {
        pub port: u16 = 6123,
    }
}

config_struct! {
    pub struct Komorebi {
        pub pipe_name: String = s("komorebi"),
    }
}

config_struct! {
    pub struct WorkSafety {
        /// Replace the wallpaper with a flat colour when its filename matches
        /// one of the keywords below.
        pub blank_wallpaper: bool = false,
        pub keywords: Vec<String> = Vec::new(),
    }
}

config_struct! {
    pub struct Hacks {
        /// Debounce, in milliseconds, between a config file write and the reload
        /// that follows it.
        pub config_reload_delay: u32 = 50,
    }
}
