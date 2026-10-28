// Locale-aware number input parsing. Money is parsed with string arithmetic
// into integer minor units, so no floating point is ever involved.

function separators(locale: string): { group: string; decimal: string } {
  const parts = new Intl.NumberFormat(locale).formatToParts(12345.6);
  return {
    group: parts.find((p) => p.type === "group")?.value ?? ",",
    decimal: parts.find((p) => p.type === "decimal")?.value ?? ".",
  };
}

/** Normalizes user input to `-123.45` form, or null when it is not a number. */
function canonical(input: string, locale: string): string | null {
  const { group, decimal } = separators(locale);
  let s = input.replace(/\s/g, "").replace(/^−/, "-");
  if (!/\s/.test(group)) s = s.split(group).join("");
  // "." is accepted as a decimal separator in every locale (keypad input).
  if (decimal !== ".") s = s.replace(decimal, ".");
  return /^-?(\d+\.?\d*|\.\d+)$/.test(s) ? s : null;
}

export function parseNumber(input: string, locale: string): number | null {
  const s = canonical(input, locale);
  return s === null ? null : Number(s);
}

/** Parses `1 234,56` into minor units (`123456` for 2 digits). */
export function parseMinor(input: string, locale: string, digits: number): number | null {
  const s = canonical(input, locale);
  if (s === null) return null;
  const negative = s.startsWith("-");
  const [int, frac = ""] = s.replace("-", "").split(".");
  if (frac.length > digits) return null;
  const minor = Number((int + frac.padEnd(digits, "0")).replace(/^0+(?=\d)/, ""));
  if (!Number.isSafeInteger(minor)) return null;
  return negative && minor !== 0 ? -minor : minor;
}

export function formatNumber(value: number, locale: string): string {
  return new Intl.NumberFormat(locale, { maximumFractionDigits: 20 }).format(value);
}

/** Formats minor units as a plain localized number (no currency symbol). */
export function formatMinor(value: number, locale: string, digits: number): string {
  const sign = value < 0 ? "-" : "";
  const abs = String(Math.abs(value)).padStart(digits + 1, "0");
  const int = abs.slice(0, abs.length - digits);
  const frac = abs.slice(abs.length - digits);
  const decimal = digits > 0 ? `${sign}${int}.${frac}` : `${sign}${int}`;
  // Intl formats decimal strings exactly, without converting to a float.
  return new Intl.NumberFormat(locale, {
    minimumFractionDigits: digits,
    maximumFractionDigits: digits,
  }).format(decimal as Intl.StringNumericLiteral);
}
