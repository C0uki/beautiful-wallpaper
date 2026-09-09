// A bounded worker pool for async side effects.
//
// The wallpaper picker used to fire one invoke() per file in the folder all
// at once — fine for a handful of pictures, but a folder with hundreds or
// thousands asks the backend to decode, resize and encode all of them in
// parallel, far past what the machine's cores can do at once, and the grid
// shows nothing until the very last one lands. This runs at most `limit` at
// a time instead, so results still arrive for every item, just as each one
// finishes rather than all together at the end.

/**
 * Runs `fn` over `items`, at most `limit` in flight at once.
 *
 * Order of completion is not the order of `items` — that is the point, since
 * a slow one at the front should not hold up the fast ones behind it.
 */
export async function forEachLimit<T>(
  items: readonly T[],
  limit: number,
  fn: (item: T) => Promise<void>,
): Promise<void> {
  let next = 0;
  async function worker() {
    while (next < items.length) {
      await fn(items[next++]!);
    }
  }
  const workers = Math.max(1, Math.min(limit, items.length));
  await Promise.all(Array.from({ length: workers }, worker));
}
