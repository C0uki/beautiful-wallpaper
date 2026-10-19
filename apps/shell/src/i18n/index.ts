// String lookup, ported from end4-pC's `services/Translation.qml`.
//
// The dictionary format is deliberately identical: a flat JSON object keyed by
// the English source string, with `%1`-style placeholders and a `/*keep*/`
// suffix marking an intentional identity translation. That means the upstream
// `translations/*.json` files can be dropped into `locales/` unchanged.

import en_US from "./locales/en_US.json";
import ja_JP from "./locales/ja_JP.json";
// Read by the native toasts too, which match the same notices.
import NOTICE_PATTERNS from "./notices.json";

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

/**
 * The shell's own notices, as the Rust side words them, `%n` standing for
 * what it fills in. It composes them in English with the values already in
 * place, so a translation is found by matching a notice against these rather
 * than by looking it up.
 */
const NOTICES = NOTICE_PATTERNS.map((notice) => ({
  notice,
  pattern: new RegExp(
    `^${notice
      .replace(/[.*+?^${}()|[\]\\]/g, "\\$&")
      .replace(/%\d/g, "(.+)")}$`,
    "s",
  ),
}));

/**
 * A notification's summary or body in the active language, when the shell
 * itself wrote it. Other applications' notifications arrive in their own
 * words and are left as they are. One line at a time: a notice can carry more
 * than one message, a line each.
 */
export function trNotice(appName: string, text: string): string {
  if (appName !== "beautiful-wallpaper") return text;
  return text
    .split("\n")
    .map((line) => {
      for (const { notice, pattern } of NOTICES) {
        const found = pattern.exec(line);
        if (found) return tr(notice, ...found.slice(1));
      }
      return tr(line);
    })
    .join("\n");
}
