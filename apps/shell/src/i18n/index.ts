// String lookup, ported from end4-pC's `services/Translation.qml`.
//
// The dictionary format is deliberately identical: a flat JSON object keyed by
// the English source string, with `%1`-style placeholders and a `/*keep*/`
// suffix marking an intentional identity translation. That means the upstream
// `translations/*.json` files can be dropped into `locales/` unchanged.

import en_US from "./locales/en_US.json";
import ja_JP from "./locales/ja_JP.json";

type Dictionary = Record<string, string>;

const DICTIONARIES: Record<string, Dictionary> = { en_US, ja_JP };

const KEEP_MARKER = "/*keep*/";

let active: Dictionary = en_US;
let activeLocale = "en_US";

/** Resolves `"auto"` against the browser/OS language. */
export function resolveLocale(requested: string): string {
  if (requested !== "auto") {
    return requested in DICTIONARIES ? requested : "en_US";
  }
  const candidates =
    typeof navigator === "undefined" ? [] : navigator.languages;
  for (const tag of candidates) {
    const normalized = tag.replace("-", "_");
    if (normalized in DICTIONARIES) return normalized;
    // `ja` should find `ja_JP`.
    const prefix = normalized.split("_")[0];
    const match = Object.keys(DICTIONARIES).find((name) =>
      name.startsWith(`${prefix}_`),
    );
    if (match) return match;
  }
  return "en_US";
}

export function setLocale(requested: string): string {
  activeLocale = resolveLocale(requested);
  active = DICTIONARIES[activeLocale] ?? en_US;
  return activeLocale;
}

export function currentLocale(): string {
  return activeLocale;
}

export function availableLocales(): string[] {
  return Object.keys(DICTIONARIES);
}

/**
 * Translates `source`, substituting `%1`, `%2`, … with `args`.
 *
 * A missing key returns the English source unchanged, so an untranslated string
 * still reads correctly rather than showing a key.
 */
export function tr(source: string, ...args: Array<string | number>): string {
  const translated = active[source] ?? source;
  const cleaned = translated.endsWith(KEEP_MARKER)
    ? translated.slice(0, -KEEP_MARKER.length)
    : translated;
  return args.reduce<string>(
    (text, value, index) => text.replaceAll(`%${index + 1}`, String(value)),
    cleaned,
  );
}
