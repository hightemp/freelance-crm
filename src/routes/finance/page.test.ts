import { cleanup, render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { invoice } from "$lib/features/finance/fixtures";
import Page from "./+page.svelte";

const pageState = vi.hoisted(() => ({ url: new URL("http://localhost/finance") }));
vi.mock("$app/state", () => ({ page: pageState }));

let invoices: unknown[] = [];

beforeEach(() => {
  invoices = [];
  mockIPC((cmd) => {
    if (cmd === "finance_dashboard")
      return { currencies: [], months: [], clients: [], openInvoices: [], taxesEnabled: false };
    if (cmd === "list_invoices") return invoices;
    if (cmd === "preview_invoice_number") return "INV-2026-002";
    return [];
  });
});

afterEach(() => {
  cleanup();
  clearMocks();
  pageState.url = new URL("http://localhost/finance");
});

describe("finance page", () => {
  it("opens on the overview and switches to invoices and expenses", async () => {
    render(Page);
    expect(screen.getByRole("heading", { level: 1 })).toHaveTextContent("Finance");
    expect(screen.getByRole("tab", { name: "Overview" })).toHaveAttribute("aria-selected", "true");
    await userEvent.click(screen.getByRole("tab", { name: "Invoices" }));
    expect(await screen.findByText("No invoices yet")).toBeInTheDocument();
    await userEvent.click(screen.getByRole("tab", { name: "Expenses" }));
    expect(await screen.findByText("No expenses in this period")).toBeInTheDocument();
  });

  it("opens an invoice linked from the calendar", async () => {
    invoices = [invoice()];
    pageState.url = new URL("http://localhost/finance?invoice=i1");
    render(Page);
    expect(screen.getByRole("tab", { name: "Invoices" })).toHaveAttribute("aria-selected", "true");
    expect(await screen.findByRole("dialog", { name: "Invoice INV-2026-001" })).toBeInTheDocument();
  });
});
