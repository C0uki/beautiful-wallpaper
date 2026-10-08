// Turning a key press into a chord, in the config's spelling.
//
// The config writes `Ctrl+Alt+Shift+Super+Key`, modifiers in that order —
// the order `bw_core::keys::normalise` puts them in — and the key under the
// name the hotkey parser takes. `KeyboardEvent.code` is that name already, for
// everything but letters, which the config has always written bare (`A`, not
// `KeyA`). `code` rather than `key` so a chord is the same key whatever the
// keyboard layout prints on it, and whatever Shift does to the character.

type Press = Pick<
  KeyboardEvent,
  "key" | "code" | "ctrlKey" | "altKey" | "shiftKey" | "metaKey"
>;

/** Keys that only modify another: pressed alone they are not a chord yet. */
const MODIFIERS = new Set([
  "Control",
  "Alt",
  "Shift",
  "Meta",
  "OS",
  "AltGraph",
]);

/** The chord a key press spells, or nothing while only modifiers are down. */
export function chordFrom(press: Press): string | null {
  if (MODIFIERS.has(press.key) || !press.code) return null;
  const key = /^Key[A-Z]$/.test(press.code) ? press.code.slice(3) : press.code;
  return [
    press.ctrlKey && "Ctrl",
    press.altKey && "Alt",
    press.shiftKey && "Shift",
    press.metaKey && "Super",
    key,
  ]
    .filter(Boolean)
    .join("+");
}

/** A chord as key caps, with the Windows key under the name on the key. */
export function capsOf(chord: string): string[] {
  return chord
    .split("+")
    .filter(Boolean)
    .map((part) => (part === "Super" ? "Win" : part));
}
