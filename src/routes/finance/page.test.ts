import { render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, describe, expect, it } from "vitest";
import Page from "./+page.svelte";

afterEach(() => clearMocks());

describe("finance page", () => {
  it("opens on the overview and switches to invoices and expenses", async () => {
    mockIPC((cmd) => {
      if (cmd === "finance_dashboard")
        return { currencies: [], months: [], clients: [], openInvoices: [], taxesEnabled: false };
      if (cmd === "expense_categories") return [];
      return [];
    });
    render(Page);
    expect(screen.getByRole("heading", { level: 1 })).toHaveTextContent("Finance");
    expect(screen.getByRole("tab", { name: "Overview" })).toHaveAttribute("aria-selected", "true");
    await userEvent.click(screen.getByRole("tab", { name: "Invoices" }));
    expect(await screen.findByText("No invoices yet")).toBeInTheDocument();
    await userEvent.click(screen.getByRole("tab", { name: "Expenses" }));
    expect(await screen.findByText("No expenses in this period")).toBeInTheDocument();
  });
});
