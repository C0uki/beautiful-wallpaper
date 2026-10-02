// Rearranging the pinned applications.

/** The key the dock groups an executable under: `bw_core::dock::normalise`. */
const key = (path: string) => path.replaceAll("/", "\\").toLowerCase();

/**
 * `pinned` with the entry for `moved` placed where `target` is now.
 *
 * The dock knows its icons by normalised path and the config keeps paths as
 * they were pinned, so both are matched the way the dock matches them. An
 * entry missing from the list leaves it as it was.
 */
export function reorder(
  pinned: string[],
  moved: string,
  target: string,
): string[] {
  const from = pinned.findIndex((path) => key(path) === key(moved));
  const to = pinned.findIndex((path) => key(path) === key(target));
  if (from < 0 || to < 0 || from === to) return pinned;
  const next = pinned.slice();
  const [entry] = next.splice(from, 1);
  next.splice(to, 0, entry!);
  return next;
}
