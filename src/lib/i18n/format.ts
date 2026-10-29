/** Formats an amount in minor units (kopecks, cents) as a localized price. */
export function formatMoney(amountMinor: number, currency: string, locale: string): string {
  const nf = new Intl.NumberFormat(locale, { style: "currency", currency });
  const digits = nf.resolvedOptions().maximumFractionDigits ?? 2;
  return nf.format(amountMinor / 10 ** digits);
}

export interface DateFormatOptions {
  timeZone?: string;
  withTime?: boolean;
}

/** Formats a unix-millisecond timestamp. */
export function formatDate(ts: number, locale: string, opts: DateFormatOptions = {}): string {
  return new Intl.DateTimeFormat(locale, {
    dateStyle: "medium",
    timeStyle: opts.withTime ? "short" : undefined,
    timeZone: opts.timeZone,
  }).format(ts);
}

/** Formats a calendar date `YYYY-MM-DD` without shifting it by the time zone. */
export function formatIsoDate(iso: string, locale: string): string {
  return formatDate(Date.parse(`${iso}T00:00:00Z`), locale, { timeZone: "UTC" });
}

/** Formats a duration as `h:mm`, dropping incomplete minutes. */
export function formatDuration(seconds: number): string {
  const totalMinutes = Math.floor(seconds / 60);
  const hours = Math.floor(totalMinutes / 60);
  const minutes = totalMinutes % 60;
  return `${hours}:${String(minutes).padStart(2, "0")}`;
}

const SIZE_UNITS = ["byte", "kilobyte", "megabyte", "gigabyte"] as const;

/** Formats a file size in binary multiples with a localized unit. */
export function formatFileSize(bytes: number, locale: string): string {
  let unit = 0;
  let value = bytes;
  while (value >= 1024 && unit < SIZE_UNITS.length - 1) {
    value /= 1024;
    unit += 1;
  }
  return new Intl.NumberFormat(locale, {
    style: "unit",
    unit: SIZE_UNITS[unit],
    unitDisplay: "short",
    maximumFractionDigits: 1,
  }).format(value);
}
