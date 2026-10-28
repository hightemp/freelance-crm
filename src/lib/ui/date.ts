// Calendar dates are plain ISO strings (`2026-10-28`) without time zones;
// arithmetic goes through UTC so DST never shifts a day.
// Times are `HH:MM` strings in 24-hour form.

const pad = (n: number) => String(n).padStart(2, "0");

function toUtc(iso: string): Date {
  const [y, m, d] = iso.split("-").map(Number);
  return new Date(Date.UTC(y, m - 1, d));
}

function fromUtc(date: Date): string {
  return `${date.getUTCFullYear()}-${pad(date.getUTCMonth() + 1)}-${pad(date.getUTCDate())}`;
}

export function isoDate(year: number, month: number, day: number): string | null {
  const date = new Date(Date.UTC(year, month - 1, day));
  date.setUTCFullYear(year); // years < 100 must not map to 19xx
  const valid =
    date.getUTCFullYear() === year && date.getUTCMonth() === month - 1 && date.getUTCDate() === day;
  return valid ? fromUtc(date) : null;
}

export function addDays(iso: string, n: number): string {
  const date = toUtc(iso);
  date.setUTCDate(date.getUTCDate() + n);
  return fromUtc(date);
}

/** Adds months, clamping the day to the target month's length. */
export function addMonths(iso: string, n: number): string {
  const [y, m, d] = iso.split("-").map(Number);
  const total = y * 12 + (m - 1) + n;
  const year = Math.floor(total / 12);
  const month = (total % 12) + 1;
  const lastDay = new Date(Date.UTC(year, month, 0)).getUTCDate();
  return isoDate(year, month, Math.min(d, lastDay))!;
}

/** `weekStart`: 0 = Sunday … 6 = Saturday. */
export function startOfWeek(iso: string, weekStart: number): string {
  const day = toUtc(iso).getUTCDay();
  return addDays(iso, -((day - weekStart + 7) % 7));
}

/** Six weeks of ISO dates covering the month, aligned to `weekStart`. */
export function monthGrid(year: number, month: number, weekStart: number): string[][] {
  let day = startOfWeek(isoDate(year, month, 1)!, weekStart);
  const weeks: string[][] = [];
  for (let w = 0; w < 6; w++) {
    const week: string[] = [];
    for (let i = 0; i < 7; i++) {
      week.push(day);
      day = addDays(day, 1);
    }
    weeks.push(week);
  }
  return weeks;
}

export function weekdayNames(locale: string, weekStart: number): string[] {
  const fmt = new Intl.DateTimeFormat(locale, { weekday: "short", timeZone: "UTC" });
  // 2023-01-01 is a Sunday.
  return Array.from({ length: 7 }, (_, i) =>
    fmt.format(Date.UTC(2023, 0, 1 + ((weekStart + i) % 7))),
  );
}

export function monthTitle(iso: string, locale: string): string {
  return new Intl.DateTimeFormat(locale, {
    month: "long",
    year: "numeric",
    timeZone: "UTC",
  }).format(toUtc(iso));
}

export function longDate(iso: string, locale: string): string {
  return new Intl.DateTimeFormat(locale, { dateStyle: "full", timeZone: "UTC" }).format(toUtc(iso));
}

/** Order of day/month/year fields in the locale's numeric date format. */
function fieldOrder(locale: string): ("day" | "month" | "year")[] {
  return new Intl.DateTimeFormat(locale, { day: "numeric", month: "numeric", year: "numeric" })
    .formatToParts(new Date())
    .map((p) => p.type)
    .filter((t): t is "day" | "month" | "year" => t === "day" || t === "month" || t === "year");
}

/** Parses manual date input in the locale's order (or ISO) to an ISO date. */
export function parseDateInput(text: string, locale: string): string | null {
  const s = text.trim();
  const iso = /^(\d{4})-(\d{1,2})-(\d{1,2})$/.exec(s);
  if (iso) return isoDate(+iso[1], +iso[2], +iso[3]);
  const parts = s.split(/[./\-\s]+/);
  if (parts.length !== 3 || !parts.every((p) => /^\d+$/.test(p))) return null;
  const fields: Record<string, number> = {};
  fieldOrder(locale).forEach((field, i) => (fields[field] = Number(parts[i])));
  const year = fields.year < 100 ? 2000 + fields.year : fields.year;
  return isoDate(year, fields.month, fields.day);
}

export function formatDateInput(iso: string, locale: string): string {
  return new Intl.DateTimeFormat(locale, {
    day: "2-digit",
    month: "2-digit",
    year: "numeric",
    timeZone: "UTC",
  }).format(toUtc(iso));
}

/** Accepts `9`, `9:05`, `0905`, `9.05`, `9:05 pm` and returns `HH:MM`. */
export function parseTimeInput(text: string): string | null {
  const match = /^(\d{1,2})(?:[:.]?(\d{2}))?\s*(am|pm)?$/i.exec(text.trim());
  if (!match) return null;
  let hours = Number(match[1]);
  const minutes = Number(match[2] ?? 0);
  const meridiem = match[3]?.toLowerCase();
  if (minutes > 59) return null;
  if (meridiem) {
    if (hours < 1 || hours > 12) return null;
    hours = (hours % 12) + (meridiem === "pm" ? 12 : 0);
  } else if (hours > 23) {
    return null;
  }
  return `${pad(hours)}:${pad(minutes)}`;
}

/** Formats `HH:MM` using the locale's clock (24h for ru, 12h for en). */
export function formatTimeInput(time: string, locale: string): string {
  const [h, m] = time.split(":").map(Number);
  const { hourCycle } = new Intl.DateTimeFormat(locale, { hour: "numeric" }).resolvedOptions();
  const twentyFour = hourCycle === "h23" || hourCycle === "h24";
  return new Intl.DateTimeFormat(locale, {
    hour: twentyFour ? "2-digit" : "numeric",
    minute: "2-digit",
    timeZone: "UTC",
  }).format(Date.UTC(2000, 0, 1, h, m));
}

/** Today's date in the user's local time zone. */
export function todayIso(): string {
  const now = new Date();
  return `${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())}`;
}
