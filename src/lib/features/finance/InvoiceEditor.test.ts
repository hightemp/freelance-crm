import { cleanup, render, screen, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { appSettings, DEFAULT_SETTINGS } from "$lib/app/settings.svelte";
import { expectNoA11yViolations } from "../../../test/a11y";
import { CLIENTS, invoice, PROJECTS } from "./fixtures";
import InvoiceEditor from "./InvoiceEditor.svelte";

let calls: { cmd: string; args: Record<string, unknown> }[] = [];

beforeEach(() => {
  calls = [];
  appSettings.value = { ...DEFAULT_SETTINGS, defaultCurrency: "RUB" };
  mockIPC((cmd, args) => {
    const a = (args ?? {}) as Record<string, unknown>;
    calls.push({ cmd, args: a });
    switch (cmd) {
      case "preview_invoice_number":
        return "INV-2026-007";
      case "create_invoice":
      case "update_invoice":
        return invoice({ id: "new" });
    }
  });
});

afterEach(() => {
  cleanup();
  clearMocks();
});

const total = () => screen.getByTestId("invoice-total").textContent?.replace(/\s/g, " ").trim();

function setup(props: Partial<Parameters<typeof InvoiceEditor>[1]> = {}) {
  const onsaved = vi.fn();
  render(InvoiceEditor, {
    clients: CLIENTS,
    projects: PROJECTS,
    today: "2026-03-10",
    onsaved,
    onclose: () => {},
    ...props,
  });
  return { onsaved, user: userEvent.setup() };
}

describe("InvoiceEditor", () => {
  it("adding and removing items recomputes the total", async () => {
    const { user } = setup({ invoice: invoice() });
    expect(total()).toBe("RUB 3,000.00");

    await user.click(screen.getByRole("button", { name: "Add item" }));
    const second = screen.getByRole("listitem", { name: "Item 2" });
    const quantity = within(second).getByRole("textbox", { name: "Quantity" });
    await user.clear(quantity);
    await user.type(quantity, "3");
    await user.type(within(second).getByRole("textbox", { name: "Price" }), "100");
    await user.tab();
    expect(total()).toBe("RUB 3,300.00");

    await user.click(screen.getByRole("button", { name: "Remove item 1" }));
    expect(total()).toBe("RUB 300.00");
  });

  it("applies discount and tax", async () => {
    const { user } = setup({ invoice: invoice() });
    await user.click(screen.getByRole("combobox", { name: "Discount" }));
    await user.click(screen.getByRole("option", { name: "Percent" }));
    await user.type(screen.getByRole("textbox", { name: "Discount, %" }), "10");
    const tax = screen.getByRole("textbox", { name: "Tax (VAT), %" });
    await user.clear(tax);
    await user.type(tax, "20");
    await user.tab();
    // 3 000 − 300 = 2 700, + 20 % = 3 240.
    expect(total()).toBe("RUB 3,240.00");
  });

  it("creates an invoice with an automatic number by default", async () => {
    const { user, onsaved } = setup();
    expect(await screen.findByPlaceholderText("INV-2026-007")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Save" }));
    expect(screen.getByText("This field is required")).toBeInTheDocument();

    await user.click(screen.getByRole("combobox", { name: /Client/ }));
    await user.click(screen.getByRole("option", { name: "Acme" }));
    const item = screen.getByRole("listitem", { name: "Item 1" });
    await user.type(within(item).getByRole("textbox", { name: "Description" }), "Design");
    await user.type(within(item).getByRole("textbox", { name: "Price" }), "50");
    await user.click(screen.getByRole("button", { name: "Save" }));

    const create = calls.find((c) => c.cmd === "create_invoice");
    expect(create?.args.input).toMatchObject({
      number: "",
      clientId: "c2",
      currency: "USD",
      issueDate: "2026-03-10",
      status: "draft",
      items: [
        {
          description: "Design",
          quantityMilli: 1000,
          price: { amountMinor: 5000, currency: "USD" },
        },
      ],
    });
    expect(onsaved).toHaveBeenCalled();
  });

  it("a paid invoice is read-only", async () => {
    const paid = invoice({
      status: "paid",
      totals: { ...invoice().totals, paid: { amountMinor: 300000, currency: "RUB" } },
    });
    setup({ invoice: paid });
    expect(screen.getByRole("status")).toHaveTextContent("can no longer be changed");
    expect(screen.getByRole("textbox", { name: "Description" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Add item" })).toBeDisabled();
    expect(screen.queryByRole("button", { name: "Save" })).toBeNull();
    expect(screen.getAllByRole("button", { name: "Close" }).length).toBeGreaterThan(0);
  });

  it("has no a11y violations", async () => {
    setup({ invoice: invoice() });
    await expectNoA11yViolations(document.body);
  });
});
