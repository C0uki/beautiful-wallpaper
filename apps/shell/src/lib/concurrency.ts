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
 * a slow one at the front should not hold up the fast ones behind it. A
 * `limit` below 1 still runs everything, one at a time, rather than doing
 * nothing. If `fn` rejects, the returned promise rejects too, but the other
 * workers are not cancelled — each keeps pulling and running further items
 * until the whole list is drained, same as if nothing had failed.
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

/**
 * Returns a `run` function that caps how many of its callers are ever
 * mid-flight at once, the same guarantee `forEachLimit` makes — but for work
 * that shows up over time rather than as one array known upfront.
 *
 * `forEachLimit` fits a folder listing, where every file is already in hand
 * when the pool starts. It does not fit a picker that only asks for a
 * thumbnail once its tile scrolls into view: calling `forEachLimit` again
 * for each newly-visible batch would run its own separate pool alongside
 * whichever earlier batches are still finishing, and the combined total
 * could run well past `limit`. A `run` shared across every visibility event
 * keeps the cap real regardless of how the work arrives.
 *
 * A rejected `fn` only fails its own `run(...)` call; every other task
 * already queued or running is unaffected.
 */
export function createLimiter(
  limit: number,
): <T>(fn: () => Promise<T>) => Promise<T> {
  const cap = Math.max(1, limit);
  let active = 0;
  const queue: Array<() => void> = [];

  function dispatch() {
    if (active >= cap) return;
    const task = queue.shift();
    if (!task) return;
    active += 1;
    task();
  }

  return function run<T>(fn: () => Promise<T>): Promise<T> {
    return new Promise<T>((resolve, reject) => {
      queue.push(() => {
        fn()
          .then(resolve, reject)
          .finally(() => {
            active -= 1;
            dispatch();
          });
      });
      dispatch();
    });
  };
}
