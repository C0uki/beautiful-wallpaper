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
 * Returns a `run` function that caps how many of its callers are ever
 * mid-flight at once.
 *
 * The cap belongs to the limiter rather than to a batch, because the picker
 * only asks for a thumbnail once its tile scrolls into view: starting a
 * fresh pool for each newly-visible batch would run it alongside whichever
 * earlier batches are still finishing, and the combined total could go well
 * past `limit`. A `run` shared across every visibility event keeps the cap
 * real regardless of how the work arrives.
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
