// Display formats of the calendar (local time).
import type { CalendarItem_Serialize as CalendarItem } from "$lib/api/bindings";
import { formatTimeInput } from "$lib/ui/date";

const pad = (n: number) => String(n).padStart(2, "0");

/** Local time of day by the locale's clock: `14:30` or `2:30 PM`. */
export function clock(ts: number, locale: string): string {
  const d = new Date(ts);
  return formatTimeInput(`${pad(d.getHours())}:${pad(d.getMinutes())}`, locale);
}

/** "Thu, Mar 19" for a `YYYY-MM-DD`. */
export function shortDay(iso: string, locale: string): string {
  return new Intl.DateTimeFormat(locale, {
    weekday: "short",
    month: "short",
    day: "numeric",
    timeZone: "UTC",
  }).format(new Date(`${iso}T00:00:00Z`));
}

/** "Mar 16 – 22, 2026" for a span of dates. */
export function dateSpan(first: string, last: string, locale: string): string {
  const fmt = new Intl.DateTimeFormat(locale, { dateStyle: "medium", timeZone: "UTC" });
  return fmt.formatRange(new Date(`${first}T00:00:00Z`), new Date(`${last}T00:00:00Z`));
}

/** The item's time prefix in lists, empty for all-day items. */
export function itemTime(item: CalendarItem, locale: string): string {
  return item.allDay ? "" : clock(item.startAt, locale);
}

/** Tailwind classes per kind of item. */
export function kindClass(item: CalendarItem): string {
  const base = item.completed ? "opacity-60 line-through " : "";
  switch (item.kind) {
    case "event":
      return `${base}bg-accent/15 text-fg hover:bg-accent/25`;
    case "task_due":
      return `${base}border-l-2 border-accent bg-surface text-fg hover:bg-surface-hover`;
    case "project_deadline":
      return `${base}border-l-2 border-danger bg-surface text-fg hover:bg-surface-hover`;
    case "invoice_due":
      return `${base}border-l-2 border-dashed border-fg bg-surface text-fg hover:bg-surface-hover`;
  }
}
