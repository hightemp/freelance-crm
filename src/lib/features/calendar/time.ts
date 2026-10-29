// Calendar views in the user's local time: which days a view shows, the
// timestamps it asks the backend for, and where items fall.
import { addDays, addMonths, monthGrid, startOfWeek } from "$lib/ui/date";

export type CalendarView = "month" | "week" | "day" | "agenda";

/** Days shown by the agenda. */
export const AGENDA_DAYS = 30;

const pad = (n: number) => String(n).padStart(2, "0");

/** Local `YYYY-MM-DD` of a timestamp. */
export function localDate(ts: number): string {
  const d = new Date(ts);
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
}

/** Local `YYYY-MM-DDTHH:MM` of a timestamp. */
export function localDateTime(ts: number): string {
  const d = new Date(ts);
  return `${localDate(ts)}T${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

/** Minutes since local midnight. */
export function minutesOfDay(ts: number): number {
  const d = new Date(ts);
  return d.getHours() * 60 + d.getMinutes();
}

/** Local midnight of a date. */
export function midnight(iso: string): number {
  const [y, m, d] = iso.split("-").map(Number);
  return new Date(y, m - 1, d).getTime();
}

export interface ViewRange {
  days: string[];
  /** Local midnight of the first day and after the last one. */
  from: number;
  to: number;
  /** Anchors of the neighbouring periods. */
  previous: string;
  next: string;
  /** `YYYY-MM` of a month view, else the first day. */
  title: string;
}

export function viewRange(view: CalendarView, anchor: string, weekStart: number): ViewRange {
  const [year, month] = anchor.split("-").map(Number);
  let days: string[];
  let step: (n: number) => string;
  switch (view) {
    case "month":
      days = monthGrid(year, month, weekStart).flat();
      step = (n) => addMonths(anchor, n);
      break;
    case "week": {
      const first = startOfWeek(anchor, weekStart);
      days = Array.from({ length: 7 }, (_, i) => addDays(first, i));
      step = (n) => addDays(anchor, 7 * n);
      break;
    }
    case "day":
      days = [anchor];
      step = (n) => addDays(anchor, n);
      break;
    case "agenda":
      days = Array.from({ length: AGENDA_DAYS }, (_, i) => addDays(anchor, i));
      step = (n) => addDays(anchor, AGENDA_DAYS * n);
      break;
  }
  return {
    days,
    from: midnight(days[0]),
    to: midnight(addDays(days[days.length - 1], 1)),
    previous: step(-1),
    next: step(1),
    title: view === "month" ? anchor.slice(0, 7) : days[0],
  };
}

interface Timing {
  allDay: boolean;
  start: string;
  end: string;
  startAt: number;
  endAt: number;
}

/** The local days an item covers (inclusive). A timed item ending exactly
 * at midnight does not cover the next day. */
export function itemDays(item: Timing): { start: string; end: string } {
  if (item.allDay) return { start: item.start, end: item.end };
  const last = Math.max(item.startAt, item.endAt - 1);
  return { start: localDate(item.startAt), end: localDate(last) };
}
