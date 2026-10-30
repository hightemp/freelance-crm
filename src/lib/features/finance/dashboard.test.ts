import { describe, expect, it } from "vitest";
import type { Dashboard } from "$lib/api/bindings";
import { monthRows, totalsFor } from "./dashboard";

const D = {
  currencies: [
    {
      currency: "RUB",
      incomeMinor: 45000,
      expensesMinor: 3000,
      profitMinor: 42000,
      unpaidMinor: 0,
      overdueMinor: 0,
      taxMinor: null,
      netMinor: null,
    },
    {
      currency: "USD",
      incomeMinor: 2000,
      expensesMinor: 500,
      profitMinor: 1500,
      unpaidMinor: 0,
      overdueMinor: 0,
      taxMinor: null,
      netMinor: null,
    },
  ],
  months: [
    { month: "2026-01", currency: "RUB", incomeMinor: 10000, expensesMinor: 0 },
    { month: "2026-01", currency: "USD", incomeMinor: 0, expensesMinor: 500 },
    { month: "2026-02", currency: "RUB", incomeMinor: 0, expensesMinor: 0 },
    { month: "2026-02", currency: "USD", incomeMinor: 2000, expensesMinor: 0 },
  ],
  clients: [],
  openInvoices: [],
  taxesEnabled: false,
} as unknown as Dashboard;

describe("monthRows", () => {
  it("takes one currency's months in major units, with a date per month", () => {
    expect(monthRows(D, "USD")).toEqual([
      { month: "2026-01", date: new Date(2026, 0, 1), income: 0, expenses: 5 },
      { month: "2026-02", date: new Date(2026, 1, 1), income: 20, expenses: 0 },
    ]);
  });

  it("uses the currency's minor units", () => {
    const yen = {
      ...D,
      months: [{ month: "2026-01", currency: "JPY", incomeMinor: 1500, expensesMinor: 0 }],
    } as unknown as Dashboard;
    expect(monthRows(yen, "JPY")[0].income).toBe(1500);
  });
});

describe("totalsFor", () => {
  it("finds the currency's totals or falls back to the first", () => {
    expect(totalsFor(D, "USD")?.incomeMinor).toBe(2000);
    expect(totalsFor(D, "EUR")?.currency).toBe("RUB");
  });
});
