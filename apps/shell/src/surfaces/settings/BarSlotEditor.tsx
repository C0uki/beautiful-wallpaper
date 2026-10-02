// One of the bar's three slots, as chips that can be dragged between them.
//
// The slots are three settings, so each keeps its own row — search finds it,
// and its path is shown like any other — but a widget dragged out of one row
// and dropped on another is a move, written to both.

import { useState } from "react";
import { Symbol } from "../../widgets";
import { tr } from "../../i18n";
import { BAR_WIDGETS } from "../bar/widgets";
import {
  BAR_SLOTS,
  moveWidget,
  removeWidget,
  type BarLayout,
  type BarSlot,
} from "./barLayout";

/** The widget in hand. One drag at a time, shared by the three rows. */
let dragging: string | null = null;

export function BarSlotEditor({
  slot,
  layout,
  onSet,
}: {
  slot: BarSlot;
  layout: BarLayout;
  onSet: (path: string, value: unknown) => void;
}) {
  // Where a drop would land: before this chip, or past the last.
  const [over, setOver] = useState<number | null>(null);
  const widgets = layout[slot];

  const write = (next: BarLayout) => {
    for (const each of BAR_SLOTS) {
      if (next[each].join() !== layout[each].join()) onSet(each, next[each]);
    }
  };
  const drop = (index: number) => {
    setOver(null);
    if (dragging) write(moveWidget(layout, dragging, slot, index));
    dragging = null;
  };
  const unused = Object.keys(BAR_WIDGETS).filter(
    (name) => !BAR_SLOTS.some((each) => layout[each].includes(name)),
  );

  return (
    <div
      className="bw-bar-slot"
      data-over={over === widgets.length}
      onDragOver={(event) => {
        event.preventDefault();
        setOver(widgets.length);
      }}
      onDragLeave={() => setOver(null)}
      onDrop={(event) => {
        event.preventDefault();
        drop(widgets.length);
      }}
    >
      {widgets.map((name, index) => (
        <span
          key={name}
          className="bw-bar-chip"
          draggable
          data-over={over === index}
          onDragStart={(event) => {
            dragging = name;
            event.dataTransfer.effectAllowed = "move";
          }}
          onDragOver={(event) => {
            event.preventDefault();
            event.stopPropagation();
            setOver(index);
          }}
          onDrop={(event) => {
            event.preventDefault();
            event.stopPropagation();
            drop(index);
          }}
          onDragEnd={() => {
            dragging = null;
            setOver(null);
          }}
        >
          {name}
          <button
            type="button"
            aria-label={tr("Remove")}
            onClick={() => write(removeWidget(layout, name))}
          >
            <Symbol name="close" size={14} />
          </button>
        </span>
      ))}

      {unused.length ? (
        <select
          value=""
          aria-label={tr("Add a widget")}
          onChange={(event) => {
            if (event.target.value)
              write(
                moveWidget(layout, event.target.value, slot, widgets.length),
              );
          }}
        >
          <option value="">{tr("Add a widget")}</option>
          {unused.map((name) => (
            <option key={name} value={name}>
              {name}
            </option>
          ))}
        </select>
      ) : null}
    </div>
  );
}
