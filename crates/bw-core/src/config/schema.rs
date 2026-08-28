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
        pub ai: Ai = Ai::default(),
        pub appearance: Appearance = Appearance::default(),
        pub audio: Audio = Audio::default(),
        pub background: Background = Background::default(),
        pub bar: Bar = Bar::default(),
        pub capture: Capture = Capture::default(),
        pub dock: Dock = Dock::default(),
        pub hacks: Hacks = Hacks::default(),
        pub keybinds: Keybinds = Keybinds::default(),
        pub language: Language = Language::default(),
        pub notifications: Notifications = Notifications::default(),
        pub osd: Osd = Osd::default(),
        pub overview: Overview = Overview::default(),
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
    /// The dock: this shell's replacement for the taskbar it can hide.
    pub struct Dock {
        pub enable: bool = false,
        pub height: u32 = 60,
        pub icon_size: u32 = 44,
        /// Full paths of the executables kept on the dock whether or not they
        /// are running. The original stores desktop-entry ids; Windows has no
        /// equivalent, and a path is the only thing that reliably identifies
        /// an application across launches.
        pub pinned_apps: Vec<String> = Vec::new(),
        /// Case-insensitive glob patterns matched against an executable's
        /// file name — `msedgewebview2.exe`, `*host.exe`. Anything matching
        /// never reaches the dock.
        ///
        /// The original calls this `ignoredAppRegexes` and takes regular
        /// expressions. A dock ignore list is a handful of file names, so this
        /// takes globs instead rather than pulling a regex engine into the
        /// portable crate — and is named for what it actually accepts.
        pub ignored: Vec<String> = Vec::new(),
        pub show_background: bool = true,
        pub show_pin_button: bool = true,
        pub show_media: bool = true,
        /// Slide out of the way until the pointer reaches the screen edge.
        pub auto_hide: bool = true,
        /// How much of the dock stays on screen while it is hidden. This is
        /// the strip the pointer has to reach, so zero would make the dock
        /// unreachable.
        pub hover_region_height: u32 = 3,
        /// Start pinned open, reserving screen space rather than hiding.
        pub pinned_on_startup: bool = false,
    }
}

config_struct! {
    /// The system-wide keys that open the shell's surfaces.
    ///
    /// The original binds these in the compositor, which on Windows does not
    /// exist: the shell registers them itself. That brings a constraint the
    /// original never had — **Windows reserves most `Win`+letter combinations
    /// for its own shell** (`Win+S`, `Win+A`, `Win+N`, `Win+W`, `Win+Space`
    /// among them) and refuses to hand them over, and a lone `Win` press
    /// always opens the Start menu. So the defaults here are chords Windows
    /// leaves alone, and none of them is the original's bare `Super`.
    ///
    /// Which combinations are refused is not documented and shifts with the
    /// Windows version and whatever else is installed, so a registration that
    /// fails says so in a notification naming the binding, rather than
    /// leaving a key that quietly does nothing.
    ///
    /// An empty value means the action has no key.
    pub struct Keybinds {
        pub enable: bool = true,
        /// `Alt+Space` follows PowerToys Run, which is what a Windows user is
        /// most likely to already have in their fingers.
        pub overview: String = s("Alt+Space"),
        pub sidebar_left: String = s("Super+Shift+A"),
        pub sidebar_right: String = s("Super+Shift+N"),
        pub wallpaper_selector: String = s("Super+Shift+W"),
        pub widget_edit_mode: String = s("Super+Shift+D"),
        /// `Win+Shift+S` is not available: Windows keeps it for the Snipping
        /// Tool and will not hand it over.
        pub capture_region: String = s("Print"),
        pub capture_ocr: String = s("Ctrl+Print"),
        pub capture_translate: String = s("Shift+Print"),
    }
}

config_struct! {
    /// Screenshots, and reading text off the screen.
    pub struct Capture {
        pub enable: bool = true,
        /// Where screenshots go. Empty means `Pictures\Screenshots`, which is
        /// where Windows itself puts them.
        pub save_path: String = String::new(),
        pub copy_to_clipboard: bool = true,
        /// A BCP-47 tag for the recogniser, or empty for the languages the
        /// user has already told Windows they read.
        ///
        /// Recognition only works for languages whose pack is installed, so
        /// naming one here that is not present leaves the feature unavailable
        /// rather than wrong.
        pub ocr_language: String = String::new(),
    }
}

config_struct! {
    /// The full-screen search overlay.
    pub struct Overview {
        pub enable: bool = true,
        /// How many applications and windows to offer. The arithmetic answer
        /// and the web-search row are never counted against this: they are
        /// one row each and both are the point of typing.
        pub max_results: u32 = 8,
        /// A URL with `%s` where the query goes. Without the placeholder the
        /// query is appended instead, so a hand-edited prefix still works.
        pub search_engine: String = s("https://www.google.com/search?q=%s"),
        pub show_windows: bool = true,
        pub show_apps: bool = true,
        /// Whether `>` runs the rest of the line.
        pub allow_run_command: bool = true,
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
        pub left: LeftSidebar = LeftSidebar::default(),
    }
}

config_struct! {
    /// The left sidebar. The original also carries an AI chat and a booru
    /// browser here; neither is built yet.
    pub struct LeftSidebar {
        pub enable: bool = true,
        /// Fraction of the screen width the panel occupies.
        pub width: f64 = 0.26,
        pub translator: Translator = Translator::default(),
        pub media: LeftMedia = LeftMedia::default(),
        pub booru: Booru = Booru::default(),
    }
}

config_struct! {
    pub struct Translator {
        pub enable: bool = true,
        /// Milliseconds of quiet before the text is sent. Translating on every
        /// keystroke would bill a request per character.
        pub delay: u32 = 300,
        /// Two-letter code, or `auto` to detect.
        pub from: String = s("auto"),
        pub to: String = s("en"),
    }
}

config_struct! {
    pub struct LeftMedia {
        pub enable: bool = true,
    }
}

config_struct! {
    /// The image-board browser.
    ///
    /// Whether the tab exists at all is `policies.weeb`, which ships at 0.
    /// These are its settings once it does.
    pub struct Booru {
        /// One of `safebooru`, `yandere`, `konachan`, `danbooru`, `gelbooru`.
        /// Safebooru by default: it is the one board that carries nothing but
        /// safe-rated work.
        pub provider: String = s("safebooru"),
        /// Lift the safe-rating filter. Off unless set deliberately, and it
        /// does nothing on a board that has only safe work to return.
        pub allow_adult: bool = false,
        pub per_page: u32 = 30,
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
    /// Anthropic's API, used by the translator today and by the chat later.
    ///
    /// The key itself is never stored here — it lives in the Windows
    /// credential manager, reached through the `keyring` crate, the same way
    /// the online wallpaper providers' keys are.
    pub struct Ai {
        pub model: String = s("claude-opus-5"),
        pub max_tokens: u32 = 4096,
        /// Let the model search the web when it needs to. Costs tokens, so it
        /// is a setting rather than always on.
        pub web_search: bool = true,
        /// Searches per turn. Without a cap a single question can run several.
        pub max_searches: u32 = 5,
        /// Show the model's summarised reasoning in its own pane.
        pub show_thinking: bool = true,
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
