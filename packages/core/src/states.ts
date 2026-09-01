// The surface open/closed flags.
//
// A direct port of end4-pC's `GlobalStates.qml`: one boolean per surface, owned
// centrally, so a hotkey, a CLI call and a click on the bar all toggle the same
// thing. Surfaces bind their visibility to these rather than to each other.

export const StateFlag = {
  WallpaperSelectorOpen: "wallpaperSelectorOpen",
  SidebarLeftOpen: "sidebarLeftOpen",
  SidebarRightOpen: "sidebarRightOpen",
  OverviewOpen: "overviewOpen",
  /** True while a region of the screen is being picked. */
  RegionSelectOpen: "regionSelectOpen",
  SettingsOpen: "settingsOpen",
  SessionOpen: "sessionOpen",
  DesktopMenuOpen: "desktopMenuOpen",
  /** True while the drop shelf is on screen and able to take a drop. */
  ShelfOpen: "shelfOpen",
  MediaControlsOpen: "mediaControlsOpen",
  OverlayOpen: "overlayOpen",
  /** True while the desktop widgets are being rearranged. */
  WidgetEditMode: "widgetEditMode",
  /** The first-run screen. Opened by the shell itself on a machine that has
   * not been through it, and by `bw wizard open` after that. */
  WizardOpen: "wizardOpen",
} as const;

export type StateFlagName = (typeof StateFlag)[keyof typeof StateFlag];

export type GlobalStates = Record<StateFlagName, boolean>;

export const defaultStates: GlobalStates = {
  wallpaperSelectorOpen: false,
  sidebarLeftOpen: false,
  sidebarRightOpen: false,
  overviewOpen: false,
  regionSelectOpen: false,
  settingsOpen: false,
  sessionOpen: false,
  desktopMenuOpen: false,
  shelfOpen: false,
  mediaControlsOpen: false,
  overlayOpen: false,
  widgetEditMode: false,
  wizardOpen: false,
};

export const stateFlagNames = Object.values(StateFlag);

export function isStateFlag(name: string): name is StateFlagName {
  return (stateFlagNames as string[]).includes(name);
}
