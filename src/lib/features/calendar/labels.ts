import { m } from "$lib/paraglide/messages";

/** Reminder offsets offered in the event form, in minutes. */
export const REMINDER_PRESETS = [0, 5, 10, 15, 30, 60, 120, 1440, 2880, 10080];

export function reminderLabel(minutes: number): string {
  if (minutes === 0) return m.reminder_at_start();
  if (minutes % 10080 === 0) return m.reminder_weeks({ count: minutes / 10080 });
  if (minutes % 1440 === 0) return m.reminder_days({ count: minutes / 1440 });
  if (minutes % 60 === 0) return m.reminder_hours({ count: minutes / 60 });
  return m.reminder_minutes({ count: minutes });
}

export function systemTimeZone(): string {
  return Intl.DateTimeFormat().resolvedOptions().timeZone || "UTC";
}

/** Civil minutes since 1970 of a local `YYYY-MM-DDTHH:MM` (or date). */
function civil(text: string): number {
  const [date, time = "00:00"] = text.split("T");
  const [y, mo, d] = date.split("-").map(Number);
  const [h, mi] = time.split(":").map(Number);
  return Date.UTC(y, mo - 1, d, h, mi) / 60_000;
}

function format(minutes: number, withTime: boolean): string {
  const iso = new Date(minutes * 60_000).toISOString();
  return withTime ? iso.slice(0, 16) : iso.slice(0, 10);
}

/** `base` moved by however much `from` became `to` (wall-clock time). */
export function shiftLocal(base: string, from: string, to: string): string {
  return format(civil(base) + civil(to) - civil(from), base.includes("T"));
}
