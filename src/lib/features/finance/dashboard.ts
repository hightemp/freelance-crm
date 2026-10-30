import type { Dashboard } from "$lib/api/bindings";

export interface MonthRow {
  /** `YYYY-MM`. */
  month: string;
  /** The first day of the month, for a time axis. */
  date: Date;
  /** Major units: roubles, dollars. */
  income: number;
  expenses: number;
}

/** Digits after the point in `currency` (2 for RUB, 0 for JPY). */
function minorDigits(currency: string): number {
  return (
    new Intl.NumberFormat("en", { style: "currency", currency }).resolvedOptions()
      .maximumFractionDigits ?? 2
  );
}

/** Income and expenses by month of one currency, for the chart. */
export function monthRows(d: Dashboard, currency: string): MonthRow[] {
  const scale = 10 ** minorDigits(currency);
  return d.months
    .filter((m) => m.currency === currency)
    .map((m) => {
      const [year, month] = m.month.split("-").map(Number);
      return {
        month: m.month,
        date: new Date(year, month - 1, 1),
        income: m.incomeMinor / scale,
        expenses: m.expensesMinor / scale,
      };
    });
}

/** Totals of `currency`, or of the first (default) currency. */
export function totalsFor(d: Dashboard, currency: string | null) {
  return d.currencies.find((c) => c.currency === currency) ?? d.currencies[0];
}
