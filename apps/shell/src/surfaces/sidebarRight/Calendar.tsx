// A month grid.
//
// Small enough to write directly rather than pull in a date library: the only
// awkward part is the leading offset, and `time.weekStartsOnMonday` decides it.

import { useState } from "react";
import { IconButton } from "../../widgets";
import { useShell } from "../../shell/store";

const WEEKDAYS_FROM_SUNDAY = ["S", "M", "T", "W", "T", "F", "S"];

/** Days in a month, with February's leap year handled by the Date rollover. */
export function daysInMonth(year: number, month: number): number {
  return new Date(year, month + 1, 0).getDate();
}

/**
 * How many blank cells precede the first of the month.
 *
 * `getDay` counts from Sunday; a week starting Monday shifts everything by one,
 * and Sunday then has to wrap to the end rather than to -1.
 */
export function leadingBlanks(
  year: number,
  month: number,
  weekStartsOnMonday: boolean,
): number {
  const weekday = new Date(year, month, 1).getDay();
  return weekStartsOnMonday ? (weekday + 6) % 7 : weekday;
}

export function Calendar() {
  const now = useShell((state) => state.now);
  const mondayFirst = useShell((state) => state.config.time.weekStartsOnMonday);
  const [offset, setOffset] = useState(0);

  const shown = new Date(now.getFullYear(), now.getMonth() + offset, 1);
  const year = shown.getFullYear();
  const month = shown.getMonth();

  const weekdays = mondayFirst
    ? [...WEEKDAYS_FROM_SUNDAY.slice(1), WEEKDAYS_FROM_SUNDAY[0]!]
    : WEEKDAYS_FROM_SUNDAY;

  const blanks = leadingBlanks(year, month, mondayFirst);
  const total = daysInMonth(year, month);
  const isThisMonth = year === now.getFullYear() && month === now.getMonth();

  return (
    <div className="bw-calendar">
      <header className="bw-calendar-header">
        <IconButton
          icon="chevron_left"
          size={30}
          label="Previous month"
          onClick={() => setOffset((value) => value - 1)}
        />
        <button
          type="button"
          className="bw-calendar-title"
          onClick={() => setOffset(0)}
        >
          {shown.toLocaleDateString(undefined, {
            month: "long",
            year: "numeric",
          })}
        </button>
        <IconButton
          icon="chevron_right"
          size={30}
          label="Next month"
          onClick={() => setOffset((value) => value + 1)}
        />
      </header>

      <div className="bw-calendar-grid" role="grid">
        {weekdays.map((day, index) => (
          <span key={index} className="bw-calendar-weekday">
            {day}
          </span>
        ))}
        {Array.from({ length: blanks }, (_, index) => (
          <span key={`blank-${index}`} />
        ))}
        {Array.from({ length: total }, (_, index) => {
          const day = index + 1;
          return (
            <span
              key={day}
              className="bw-calendar-day"
              data-today={isThisMonth && day === now.getDate()}
            >
              {day}
            </span>
          );
        })}
      </div>
    </div>
  );
}
