// The contract between the Rust backend and every surface.
//
// Config, theme and wallpaper shapes are generated from the Rust types so the
// two halves cannot drift; the event and command names in `ipc` are the part
// that has to be agreed by hand. Regenerate everything with `pnpm gen:types`.

import defaultConfigJson from "./generated/defaultConfig.json";
import type { Config } from "./generated/Config";

export * from "./generated-index";
export * from "./ipc";
export * from "./states";

/** The schema's defaults, emitted from the same Rust struct the backend uses. */
export const defaultConfig = defaultConfigJson as unknown as Config;
