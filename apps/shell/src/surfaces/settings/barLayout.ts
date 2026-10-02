// Moving widgets between the bar's three slots.

/** The three slots, as config paths, left to right. */
export const BAR_SLOTS = ["bar.left", "bar.center", "bar.right"] as const;
export type BarSlot = (typeof BAR_SLOTS)[number];
export type BarLayout = Record<BarSlot, string[]>;

/**
 * `layout` with `widget` taken out of whichever slot holds it and put into
 * `to` before position `index` (the end, past the last). A widget is in at
 * most one slot afterwards, so dragging one cannot duplicate it.
 */
export function moveWidget(
  layout: BarLayout,
  widget: string,
  to: BarSlot,
  index: number,
): BarLayout {
  const next = { ...layout };
  let at = index;
  for (const slot of BAR_SLOTS) {
    const found = next[slot].indexOf(widget);
    if (found < 0) continue;
    next[slot] = next[slot].filter((each) => each !== widget);
    // Taking it out of the same slot first shifts everything after it left.
    if (slot === to && found < at) at -= 1;
  }
  const target = next[to].slice();
  target.splice(Math.min(Math.max(at, 0), target.length), 0, widget);
  next[to] = target;
  return next;
}

/** `layout` with `widget` taken out of every slot. */
export function removeWidget(layout: BarLayout, widget: string): BarLayout {
  const next = { ...layout };
  for (const slot of BAR_SLOTS) {
    next[slot] = next[slot].filter((each) => each !== widget);
  }
  return next;
}
