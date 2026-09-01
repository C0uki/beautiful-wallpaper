// How a change is shown when both sides are long and begin the same way.
//
// Nearly every long value in this config is a file path, and two paths cut to
// the width of a column are the same characters twice:
//
//     C:/Users/you/Pictures/…  →  C:/Users/you/Pictur…
//
// which is a row saying a setting would change and then showing no
// difference — worse than no row, because it reads as a bug. Ellipsizing from
// the other end would fix it, but `direction: rtl` moves punctuation around in
// values that are not paths and Chromium ignores it under `unicode-bidi:
// plaintext` anyway, so the trimming is done here where it can be tested.
//
// Nothing is guessed about the type of the value: what makes the two ends of a
// row informative is that they start where they stop agreeing, whatever they
// are.

/** Beyond this a value cannot be read in the column anyway. */
const LIMIT = 26;

/** Where a tail may start, so it begins at a whole segment. */
const BOUNDARIES = "/\\.,: -_";

/** At least this much shared beginning before dropping any of it is worth it. */
const WORTH_IT = 8;

/** The two sides of a change, trimmed to where they differ. */
export function contrast(from: string, to: string): [string, string] {
  if (from.length <= LIMIT && to.length <= LIMIT) return [from, to];

  let shared = 0;
  while (
    shared < from.length &&
    shared < to.length &&
    from[shared] === to[shared]
  ) {
    shared += 1;
  }

  // Back up to a boundary: a tail starting in the middle of a folder name is
  // harder to read than the whole path.
  while (shared > 0 && !BOUNDARIES.includes(from[shared - 1]!)) shared -= 1;
  if (shared < WORTH_IT) return [from, to];

  return [`…${from.slice(shared)}`, `…${to.slice(shared)}`];
}
