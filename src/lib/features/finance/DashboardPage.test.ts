import { cleanup, render, screen, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import type { Dashboard } from "$lib/api/bindings";
import { expectNoA11yViolations } from "../../../test/a11y";
import DashboardPage from "./DashboardPage.svelte";

let calls: { cmd: string; args: Record<string, unknown> }[] = [];
let dashboard: Dashboard;

const totals = (currency: string, income: number, patch = {}) => ({
  currency,
  incomeMinor: income,
  expensesMinor: 1000,
  profitMinor: income - 1000,
  unpaidMinor: 50000,
  overdueMinor: 20000,
  taxMinor: null,
  netMinor: null,
  ...patch,
});

beforeEach(() => {
  calls = [];
  dashboard = {
    currencies: [totals("RUB", 450000), totals("USD", 2000)],
    months: [
      { month: "2026-01", currency: "RUB", incomeMinor: 100000, expensesMinor: 0 },
      { month: "2026-01", currency: "USD", incomeMinor: 0, expensesMinor: 1000 },
      { month: "2026-02", currency: "RUB", incomeMinor: 350000, expensesMinor: 1000 },
      { month: "2026-02", currency: "USD", incomeMinor: 2000, expensesMinor: 0 },
    ],
    clients: [
      { clientId: "c1", clientName: "Ромашка", currency: "RUB", incomeMinor: 450000 },
      { clientId: "c2", clientName: "Acme", currency: "USD", incomeMinor: 2000 },
    ],
    openInvoices: [
      {
        id: "i1",
        number: "INV-2026-001",
        clientId: "c1",
        clientName: "Ромашка",
        dueDate: "2026-02-01",
        due: { amountMinor: 20000, currency: "RUB" },
        overdue: true,
      },
    ],
    taxesEnabled: false,
  } as unknown as Dashboard;
  mockIPC((cmd, args) => {
    calls.push({ cmd, args: (args ?? {}) as Record<string, unknown> });
    if (cmd === "finance_dashboard") return dashboard;
  });
});

afterEach(() => {
  cleanup();
  clearMocks();
});

const tile = (name: string) => screen.getByRole("group", { name });

describe("DashboardPage", () => {
  it("asks for the year so far and shows the default currency", async () => {
    render(DashboardPage, { today: "2026-03-10" });
    expect(await screen.findByRole("group", { name: "Income" })).toHaveTextContent("RUB 4,500.00");
    expect(tile("Expenses")).toHaveTextContent("RUB 10.00");
    expect(tile("Profit")).toHaveTextContent("RUB 4,490.00");
    expect(tile("Unpaid")).toHaveTextContent("RUB 500.00");
    expect(tile("Overdue")).toHaveTextContent("RUB 200.00");
    expect(screen.queryByRole("group", { name: "Tax" })).toBeNull();
    expect(calls[0].args.period).toEqual({ from: "2026-01-01", to: "2026-03-10" });
  });

  it("switching the currency changes totals, the chart and clients", async () => {
    render(DashboardPage, { today: "2026-03-10" });
    await screen.findByRole("group", { name: "Income" });
    const chart = screen.getByRole("table", { name: "By month" });
    expect(within(chart).getAllByRole("row")[1]).toHaveTextContent("RUB 1,000.00");

    await userEvent.click(screen.getByRole("radio", { name: "USD" }));
    expect(tile("Income")).toHaveTextContent("$20.00");
    const rows = within(screen.getByRole("table", { name: "By month" })).getAllByRole("row");
    expect(rows.map((r) => r.textContent)).toEqual([
      expect.stringContaining("Income"),
      expect.stringMatching(/\$0\.00.*\$10\.00/),
      expect.stringMatching(/\$20\.00.*\$0\.00/),
    ]);
    const clients = screen.getByRole("list", { name: "By client" });
    expect(within(clients).getAllByRole("listitem")).toHaveLength(1);
    expect(within(clients).getByText("Acme")).toBeInTheDocument();
  });

  it("a client leads to their finances", async () => {
    render(DashboardPage, { today: "2026-03-10" });
    const link = await screen.findByRole("link", { name: /Ромашка/ });
    expect(link).toHaveAttribute("href", "/clients/c1?tab=finance");
  });

  it("lists open invoices with overdue ones marked", async () => {
    render(DashboardPage, { today: "2026-03-10" });
    const list = await screen.findByRole("list", { name: "Open invoices" });
    expect(within(list).getByText("INV-2026-001")).toBeInTheDocument();
    expect(within(list).getByText("Overdue")).toBeInTheDocument();
  });

  it("shows tax and net income when taxes are on", async () => {
    dashboard.taxesEnabled = true;
    dashboard.currencies[0] = totals("RUB", 450000, { taxMinor: 27000, netMinor: 423000 });
    render(DashboardPage, { today: "2026-03-10" });
    expect(await screen.findByRole("group", { name: "Tax" })).toHaveTextContent("RUB 270.00");
    expect(tile("After tax")).toHaveTextContent("RUB 4,230.00");
  });

  it("has no a11y violations", async () => {
    render(DashboardPage, { today: "2026-03-10" });
    await screen.findByRole("group", { name: "Income" });
    await expectNoA11yViolations(document.body);
  });
});
