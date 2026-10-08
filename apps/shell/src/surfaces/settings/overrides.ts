// Where the generated form is not good enough on its own.
//
// The schema knows a value is a string; it does not know the string is one of
// four bar styles. It knows a value is a decimal; it does not know the decimal
// is a fraction of the screen and wants a slider rather than a box to type a
// number into. And its labels are mechanical — `from` reads the same for a
// night-light schedule and a translation — with nothing to say what a setting
// does. So four things are curated here: choices, ranges, labels and the
// sentence under a label.
//
// Anything absent from here still gets a control, so this file is an
// improvement on the default rather than a list that has to be kept complete.

import { tr } from "../../i18n";
import { TRANSITION_NAMES } from "../../gl/transitions";

export interface Choice {
  value: string;
  label: () => string;
}

export interface Override {
  /** Offer these instead of a free-text box. */
  choices?: Choice[];
  /** Draw a slider between these instead of a number box. */
  range?: { min: number; max: number; step: number };
  /** A sentence under the label, where the setting needs one. */
  hint?: () => string;
  /** A label for where the mechanical one reads wrongly. */
  label?: () => string;
  /** One of the bar's three slots: chips dragged between the three rows. */
  barSlot?: true;
}

/** Values that are really a choice, keyed by config path. */
const plain = (values: string[]): Choice[] =>
  values.map((value) => ({ value, label: () => value }));

/** Choices whose stored spelling is not something to show: each value with
 *  the English words for it, translated when drawn. */
const named = (pairs: [string, string][]): Choice[] =>
  pairs.map(([value, words]) => ({ value, label: () => tr(words) }));

/** What a screen corner can open. */
const CORNER_ACTIONS = named([
  ["", "Do nothing"],
  ["sidebarLeftOpen", "Left sidebar"],
  ["sidebarRightOpen", "Right sidebar"],
  ["overviewOpen", "Overview"],
  ["wallpaperSelectorOpen", "Wallpaper picker"],
  ["sessionOpen", "Session screen"],
  ["shelfOpen", "Shelf"],
  ["overlayOpen", "Overlay"],
  ["settingsOpen", "Settings"],
]);

export const OVERRIDES: Record<string, Override> = {
  "language.ui": {
    choices: [
      { value: "auto", label: () => tr("Automatic") },
      { value: "en_US", label: () => "English" },
      { value: "ja_JP", label: () => "日本語" },
    ],
  },
  "appearance.palette.type": {
    choices: plain([
      "auto",
      "tonalSpot",
      "neutral",
      "vibrant",
      "expressive",
      "content",
      "fidelity",
      "monochrome",
      "rainbow",
      "fruitSalad",
    ]),
    hint: () => tr("`auto` picks a variant to suit each wallpaper."),
  },
  "appearance.palette.mode": {
    choices: named([
      ["auto", "Automatic"],
      ["light", "Light"],
      ["dark", "Dark"],
    ]),
  },
  "appearance.roundingScale": { range: { min: 0, max: 2, step: 0.05 } },
  "appearance.transparency.extra": { range: { min: 0, max: 1, step: 0.05 } },
  // Tri-state numbers. A box showing "1" is honest and says nothing.
  "policies.ai": {
    choices: [
      { value: "0", label: () => tr("Off") },
      { value: "1", label: () => tr("On") },
      { value: "2", label: () => tr("Local models only") },
    ],
  },
  "policies.weeb": {
    choices: [
      { value: "0", label: () => tr("Off") },
      { value: "1", label: () => tr("On") },
    ],
  },
  "bar.style": { choices: plain(["m3", "hug", "float", "islands"]) },
  "bar.left": { barSlot: true },
  "bar.center": { barSlot: true },
  "bar.right": { barSlot: true },
  "background.wallpaperAnimation": {
    choices: plain([...TRANSITION_NAMES, "random"]),
  },
  // Every desktop widget's: kept where it was put, or moved to wherever
  // the wallpaper is calmest.
  ...Object.fromEntries(
    [
      "clock",
      "media",
      "weather",
      "resources",
      "calendar",
      "userCard",
      "notes",
    ].map((id) => [
      `background.widgets.${id}.placementStrategy`,
      {
        choices: named([
          ["free", "Where I put it"],
          ["leastBusy", "The calmest part of the wallpaper"],
        ]),
      },
    ]),
  ),
  "background.parallax.zoom": { range: { min: 1, max: 1.5, step: 0.01 } },
  "background.parallax.workspacePan": { range: { min: 0, max: 1, step: 0.05 } },
  "sidebar.width": { range: { min: 0.15, max: 0.6, step: 0.01 } },
  "sidebar.left.width": { range: { min: 0.15, max: 0.6, step: 0.01 } },
  "sidebar.quickToggles.style": {
    choices: named([
      ["android", "Grid of tiles"],
      ["classic", "One row of buttons"],
    ]),
  },
  "shelf.width": { range: { min: 0.1, max: 0.5, step: 0.01 } },
  "shelf.edge": {
    choices: named([
      ["left", "Left"],
      ["right", "Right"],
    ]),
  },
  "osd.position": {
    choices: named([
      ["top", "Top edge"],
      ["bottom", "Bottom edge"],
    ]),
  },
  // Spelled as the backend spells them: `Notifications::position` is a plain
  // string the Rust side matches with `ends_with("left")`, and its default is
  // `top_right`. camelCase here would leave the box showing no value at all.
  "notifications.position": {
    choices: named([
      ["top_left", "Top left"],
      ["top_center", "Top centre"],
      ["top_right", "Top right"],
      ["bottom_left", "Bottom left"],
      ["bottom_center", "Bottom centre"],
      ["bottom_right", "Bottom right"],
    ]),
  },
  "overlay.clickthroughOpacity": { range: { min: 0.1, max: 1, step: 0.05 } },
  "overlay.crosshair.code": {
    hint: () =>
      tr("A Valorant crosshair share code, from the game or a builder site."),
  },
  "background.widgets.clock.style": {
    choices: named([
      ["default", "Analogue"],
      ["digital", "Digital"],
    ]),
  },
  "windows.backdrop": {
    choices: named([
      ["auto", "Automatic"],
      ["mica", "Mica"],
      ["acrylic", "Acrylic"],
      ["none", "None"],
    ]),
  },
  "wallpaperSelector.online.defaultProvider": {
    choices: plain(["wallhaven", "unsplash", "pexels"]),
  },
  "wallpaperSelector.online.resolution": {
    choices: plain(["1080p", "2k", "4k"]),
  },
  "wallpaperSelector.online.purity": {
    choices: named([
      ["sfw", "Safe for work"],
      ["sketchy", "Suggestive"],
      ["nsfw", "Adult"],
    ]),
  },
  "wallpaperSelector.online.category": {
    choices: named([
      ["general", "General"],
      ["anime", "Anime"],
      ["people", "People"],
      ["all", "All"],
    ]),
  },
  "sidebar.left.booru.provider": {
    choices: plain([
      "safebooru",
      "yandere",
      "konachan",
      "danbooru",
      "gelbooru",
    ]),
  },
  "sidebar.cornerOpen.topLeftAction": { choices: CORNER_ACTIONS },
  "sidebar.cornerOpen.topRightAction": { choices: CORNER_ACTIONS },
  "sidebar.nightLight.temperature": {
    range: { min: 1200, max: 6500, step: 100 },
  },
};

/** Labels the mechanical ones get wrong: the same word meaning two things,
 *  or a word that only means something beside its group's name. English,
 *  translated when drawn. */
const LABELS: Record<string, string> = {
  "sidebar.left.translator.from": "Translate from",
  "sidebar.left.translator.to": "Translate into",
  "sidebar.left.translator.delay": "Wait before translating",
  "sidebar.nightLight.from": "Turns on at",
  "sidebar.nightLight.to": "Turns off at",
  "overlay.crosshair.code": "Crosshair code",
  "policies.ai": "AI features",
  "policies.weeb": "Anime and image-board features",
  "background.widgets.grid": "Snap to a grid",
  "appearance.fonts.pixelSize": "Text size",
  "hacks.desktopMenu": "Take over the desktop's right-click menu",
  "sidebar.cornerOpen.enable": "Open things from the screen corners",
};

/** The sentence under a label, for every setting whose name alone does not
 *  say what it does. English, translated when drawn. */
const HINTS: Record<string, string> = {
  "language.ui": "Automatic follows the language Windows is set to.",
  "time.format":
    "HH for hours, mm for minutes, ss for seconds. HH:mm shows 14:05.",
  "time.dateFormat":
    "ddd for the day's name, dd for the day, MM for the month.",
  "weather.city": "Empty finds where you are from your internet connection.",
  "weather.refreshInterval": "Seconds between updates.",
  "windows.startWithWindows": "Start the shell when you sign in.",
  "windows.autoUpdate":
    "Check for a new version a minute after starting and every six hours, and install it.",
  "appearance.palette.accentColor":
    "A colour such as #4f8cff to build the colours from. Empty takes it from the wallpaper.",
  "appearance.palette.mode": "Automatic picks by how bright the wallpaper is.",
  "appearance.fonts.main":
    "Font names as Windows lists them. Later names are used when an earlier one is missing.",
  "appearance.fonts.reading": "Used for long passages, such as AI answers.",
  "appearance.fonts.expressive": "Used for large text, such as the clock.",
  "appearance.fonts.pixelSize": "Everything else is sized from this.",
  "appearance.transparency.extra":
    "How much more see-through panels are, on top of what the wallpaper calls for.",
  "appearance.wallpaperTheming.syncSystemAccent":
    "Set Windows' accent colour and light or dark mode from the wallpaper too.",
  "appearance.wallpaperTheming.syncWindowsTerminal":
    "Write matching colours into Windows Terminal.",
  "appearance.roundingScale": "1 is the usual roundness, 0 is square corners.",
  "windows.backdrop":
    "The blur behind panels. Automatic uses Mica on Windows 11 and Acrylic on Windows 10.",
  "background.wallpaperPath":
    "The picture or video on screen. The wallpaper picker sets this.",
  "background.thumbnailPath":
    "A still from a video wallpaper, taken for its colours. Set for you.",
  "background.wallpaperAnimation": "How one wallpaper gives way to the next.",
  "background.transitionDuration": "Milliseconds the change takes.",
  "background.parallax.enable":
    "Shift the wallpaper a little as you move between workspaces.",
  "background.parallax.zoom":
    "How much the wallpaper is enlarged, so moving it never shows an edge.",
  "background.parallax.workspacePan": "How far it shifts per workspace.",
  "wallpaperSelector.userPath":
    "The folder the picker shows. Empty uses your Pictures folder.",
  "wallpaperSelector.changeInterval":
    "Seconds between changing to another wallpaper on its own. 0 never changes it.",
  "wallpaperSelector.extensions": "The file types counted as wallpapers.",
  "wallpaperSelector.online.downloadPath":
    "Where downloaded wallpapers are saved. Empty uses Pictures\\Wallpapers.",
  "background.widgets.enable":
    "Rearrange them in edit mode, from the desktop menu.",
  "background.widgets.grid":
    "Pixels between grid lines when dragging. 0 places them freely.",
  "desktopMenu.enable":
    "The menu opened by its key, or by right-clicking the desktop when that is taken over under Advanced.",
  "bar.reserveSpace":
    "Keep maximised windows clear of the bar. Ignored while it hides itself.",
  "bar.autoHide": "Slide off the edge until the pointer reaches it.",
  "bar.perMonitor": "A bar on every monitor rather than only the main one.",
  "bar.hoverRegionHeight":
    "Pixels left showing while it is hidden, for the pointer to find.",
  "bar.showFrame": "A thin border around the whole screen.",
  "bar.frameColor":
    "A colour such as #000000, or a palette name such as primary.",
  "dock.pinnedApps": "Full paths of programs, one per line.",
  "dock.ignored":
    "File names never shown on the dock, one per line. * matches anything.",
  "dock.hoverRegionHeight":
    "Pixels left showing while it is hidden, for the pointer to find.",
  "dock.pinnedOnStartup": "Start shown and keep windows clear of it.",
  "windows.hideSystemTaskbar":
    "Hide the Windows taskbar while the shell is running.",
  "sidebar.width": "Share of the screen's width.",
  "sidebar.left.width": "Share of the screen's width.",
  "sidebar.banner":
    "The wallpaper strip with your picture and uptime, at the top of the right sidebar.",
  "sidebar.bannerImage": "Empty uses the wallpaper.",
  "sidebar.profile.displayName": "Empty uses your Windows account name.",
  "sidebar.profile.avatarPath": "Empty uses your Windows account picture.",
  "sidebar.cornerOpen.valueScroll":
    "Scroll on a left corner for brightness, on a right corner for volume.",
  "sidebar.cornerOpen.clickless":
    "Open by just reaching the corner. Quicker, and easy to set off by accident.",
  "sidebar.cornerOpen.cornerRegionWidth": "Pixels along the top edge.",
  "sidebar.cornerOpen.cornerRegionHeight": "Pixels down from the top edge.",
  "sidebar.cornerOpen.visualize":
    "Colour the corner areas, to see where they are.",
  "notifications.timeout":
    "Milliseconds a notification stays up. Urgent ones stay until dismissed.",
  "notifications.maxVisible":
    "More than this wait in the notification centre without popping up.",
  "notifications.doNotDisturb":
    "Keep notifications in the centre without popping them up.",
  "notifications.width": "Pixels.",
  "sidebar.nightLight.temperature":
    "Kelvin. 6500 is no change; lower is warmer.",
  "sidebar.nightLight.automatic": "Turn it on and off at the times below.",
  "sidebar.nightLight.from": "24-hour time, such as 20:00.",
  "sidebar.nightLight.to": "24-hour time, such as 07:00.",
  "osd.enable": "Show the level when a volume or brightness key is pressed.",
  "osd.timeout": "Milliseconds it stays after the last press.",
  "audio.step": "How much one press of a volume key changes it, in percent.",
  "audio.protection.enable":
    "Stop the volume being turned up past the limit below.",
  "audio.protection.maxVolume": "Percent.",
  "sidebar.left.media.lyrics":
    "Look up the lyrics online; the track's title and artist are sent to lrclib.net.",
  "overview.maxResults": "Apps and windows listed at once.",
  "overview.searchEngine":
    "The web address to search with, %s marking where the words go.",
  "overview.allowRunCommand": "Typing > followed by a command runs it.",
  "capture.savePath": "Empty uses Pictures\\Screenshots, as Windows does.",
  "capture.ocrLanguage":
    "A language code such as ja or en-US. Empty uses the languages Windows is set up for.",
  "shelf.width": "Share of the screen's width.",
  "shelf.maxItems":
    "Files past this are turned away rather than pushing others off.",
  "shelf.clearAfterDrag":
    "Take a file off the shelf once it has been dragged out.",
  "overlay.darkenScreen": "Dim what is behind while the overlay is open.",
  "overlay.clickthroughOpacity":
    "How solid a pinned panel looks while clicks go through it.",
  "session.force": "Close programs without letting them save. Best left off.",
  "ai.model": "The Claude model that answers, such as claude-opus-5.",
  "ai.maxTokens": "The longest answer, in tokens.",
  "ai.webSearch":
    "Let it search the web when it needs to. Searches cost tokens.",
  "ai.maxSearches": "Searches allowed per question.",
  "ai.showThinking": "Show a summary of how it reached its answer.",
  "policies.ai": "Local models only keeps everything on this computer.",
  "sidebar.left.translator.delay":
    "Milliseconds after you stop typing, so each keystroke is not a request.",
  "sidebar.left.translator.from": "A two-letter code such as en, or auto.",
  "sidebar.left.translator.to": "A two-letter code such as ja.",
  "workSafety.blankWallpaper":
    "Show a plain colour instead when the wallpaper's file name contains a keyword below.",
  "workSafety.keywords": "One per line.",
  "policies.weeb": "Shows the image-board tab in the left sidebar.",
  "sidebar.left.booru.allowAdult":
    "Lift the safe-only filter. Has no effect on Safebooru.",
  "sidebar.left.booru.perPage": "Images loaded at a time.",
  "hacks.configReloadDelay":
    "Milliseconds to wait after the settings file changes before reading it.",
  "hacks.desktopMenu":
    "Catches every right-click on the desktop with a system-wide mouse hook.",
  "hacks.readOtherNotifications":
    "Needs the shell installed with a package identity. See docs/msix.md.",
  "resources.pollInterval":
    "Milliseconds between readings of CPU, memory and disk.",
  "resources.showSwap": "Show the page file alongside memory.",
  "windows.glazewm.port": "Where GlazeWM listens, for the workspaces widget.",
};

for (const [path, words] of Object.entries(LABELS)) {
  OVERRIDES[path] = { ...OVERRIDES[path], label: () => tr(words) };
}
for (const [path, words] of Object.entries(HINTS)) {
  // A hint already written above, with its own reasoning, stays.
  OVERRIDES[path] = { hint: () => tr(words), ...OVERRIDES[path] };
}
