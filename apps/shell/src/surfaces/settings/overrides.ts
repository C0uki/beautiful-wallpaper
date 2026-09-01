// Where the generated form is not good enough on its own.
//
// The schema knows a value is a string; it does not know the string is one of
// four bar styles. It knows a value is a decimal; it does not know the decimal
// is a fraction of the screen and wants a slider rather than a box to type a
// number into. Those are the two things worth curating, and they are the only
// two — everything else the generated control gets right.
//
// Anything absent from here still gets a control, so this file is an
// improvement on the default rather than a list that has to be kept complete.

import { tr } from "../../i18n";

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
}

/** Values that are really a choice, keyed by config path. */
const plain = (values: string[]): Choice[] =>
  values.map((value) => ({ value, label: () => value }));

export const OVERRIDES: Record<string, Override> = {
  "language.ui": {
    choices: [
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
  "appearance.palette.mode": { choices: plain(["auto", "light", "dark"]) },
  "appearance.roundingScale": { range: { min: 0, max: 2, step: 0.05 } },
  "appearance.transparency.extra": { range: { min: 0, max: 1, step: 0.05 } },
  "appearance.fakeScreenRounding": {
    choices: [
      { value: "0", label: () => tr("Never") },
      { value: "1", label: () => tr("Always") },
      { value: "2", label: () => tr("When nothing is full-screen") },
    ],
  },
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
  "background.wallpaperAnimation": {
    choices: plain([
      "fade",
      "circle",
      "dissolve",
      "pixelate",
      "ripple",
      "stripes",
      "random",
    ]),
  },
  "background.centeredWallpaperSize": {
    range: { min: 0.1, max: 1, step: 0.05 },
  },
  "background.parallax.zoom": { range: { min: 1, max: 1.5, step: 0.01 } },
  "background.parallax.workspacePan": { range: { min: 0, max: 1, step: 0.05 } },
  "sidebar.width": { range: { min: 0.15, max: 0.6, step: 0.01 } },
  "sidebar.left.width": { range: { min: 0.15, max: 0.6, step: 0.01 } },
  "sidebar.quickToggles.style": { choices: plain(["android", "classic"]) },
  "shelf.width": { range: { min: 0.1, max: 0.5, step: 0.01 } },
  "shelf.edge": { choices: plain(["left", "right"]) },
  "osd.position": { choices: plain(["top", "bottom"]) },
  "notifications.position": {
    choices: plain([
      "topLeft",
      "topCenter",
      "topRight",
      "bottomLeft",
      "bottomCenter",
      "bottomRight",
    ]),
  },
  "overlay.clickthroughOpacity": { range: { min: 0.1, max: 1, step: 0.05 } },
  "overlay.crosshair.code": {
    hint: () =>
      tr("A Valorant crosshair share code, from the game or a builder site."),
  },
};
