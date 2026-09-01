// The contract between the Rust backend and every surface.
//
// Config, theme and wallpaper shapes are generated from the Rust types so the
// two halves cannot drift; the event and command names in `ipc` are the part
// that has to be agreed by hand. Regenerate everything with `pnpm gen:types`.

import defaultConfigJson from "./generated/defaultConfig.json";
import launcherActionsJson from "./generated/launcherActions.json";
import configSchemaJson from "./generated/configSchema.json";
import type { Config } from "./generated/Config";
import type { Field } from "./generated/Field";

export * from "./generated-index";
export * from "./ipc";
export * from "./states";

/** The schema's defaults, emitted from the same Rust struct the backend uses. */
export const defaultConfig = defaultConfigJson as unknown as Config;

/** Every `/` keyword the launcher offers, generated from `bw-core`.
 *
 * The frontend decides what each one does; this is the list it has to cover,
 * and a test asserts that it does. */
export const launcherActions: readonly string[] = launcherActionsJson;

/** Every editable config value, generated from the Rust schema.
 *
 * The settings screen builds its form from this rather than from a
 * hand-written control per key, so a key added to the schema has a control the
 * moment it exists. */
export const configSchema: readonly Field[] = configSchemaJson as Field[];
