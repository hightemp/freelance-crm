import { cleanup, render, screen, waitFor, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import type { Invoice_Serialize as Invoice } from "$lib/api/bindings";
import { appSettings, DEFAULT_SETTINGS } from "$lib/app/settings.svelte";
import DialogHost from "$lib/ui/DialogHost.svelte";
import { resetDialogs } from "$lib/ui/dialogs.svelte";
import ToastHost from "$lib/ui/ToastHost.svelte";
import { expectNoA11yViolations } from "../../../test/a11y";
import { CLIENTS, invoice, PROJECTS } from "./fixtures";
import InvoicesPage from "./InvoicesPage.svelte";

let calls: { cmd: string; args: Record<string, unknown> }[] = [];
let invoices: Invoice[] = [];

beforeEach(() => {
  calls = [];
  resetDialogs();
  appSettings.value = { ...DEFAULT_SETTINGS, defaultCurrency: "RUB" };
  invoices = [
    invoice(),
    invoice({
      id: "i2",
      number: "INV-2026-002",
      clientId: "c2",
      clientName: "Acme",
      currency: "USD",
      status: "overdue",
      dueDate: "2026-03-01",
      totals: {
        ...invoice().totals,
        total: { amountMinor: 50000, currency: "USD" },
        due: { amountMinor: 50000, currency: "USD" },
      },
    }),
  ];
  mockIPC((cmd, args) => {
    const a = (args ?? {}) as Record<string, unknown>;
    calls.push({ cmd, args: a });
    switch (cmd) {
      case "list_invoices":
        return invoices;
      case "list_clients":
        return CLIENTS;
      case "list_projects":
        return PROJECTS;
      case "preview_invoice_number":
        return "INV-2026-003";
      case "create_payment":
        invoices = [invoice({ status: "partially_paid" }), invoices[1]];
        return { payment: {}, overpaid: null };
      case "plugin:dialog|save":
        return "/home/me/INV-2026-001.pdf";
      case "save_invoice_pdf":
      case "open_invoice_pdf":
      case "delete_invoice":
        return null;
    }
  });
});

afterEach(() => {
  cleanup();
  clearMocks();
});

function setup() {
  render(InvoicesPage, { today: "2026-03-10" });
  render(DialogHost);
  render(ToastHost);
  return userEvent.setup();
}

async function actions(user: ReturnType<typeof userEvent.setup>, number: string, item: string) {
  await user.click(await screen.findByRole("button", { name: `Actions for invoice ${number}` }));
  await user.click(screen.getByRole("menuitem", { name: item }));
}

describe("InvoicesPage", () => {
  it("lists invoices with their status and total", async () => {
    setup();
    const row = (await screen.findByText("INV-2026-002")).closest("tr")!;
    expect(within(row).getByText("Acme")).toBeInTheDocument();
    expect(within(row).getByText("Overdue")).toBeInTheDocument();
    expect(within(row).getByText("$500.00")).toBeInTheDocument();
  });

  it("records a partial payment", async () => {
    const user = setup();
    await actions(user, "INV-2026-001", "Record payment");
    const dialog = screen.getByRole("dialog", { name: "Payment for invoice INV-2026-001" });
    const amount = within(dialog).getByRole("textbox", { name: "Amount" });
    expect(amount).toHaveValue("3,000.00");
    await user.clear(amount);
    await user.type(amount, "1000");
    await user.click(within(dialog).getByRole("button", { name: "Save" }));
    await waitFor(() =>
      expect(calls.find((c) => c.cmd === "create_payment")?.args.input).toMatchObject({
        invoiceId: "i1",
        amount: { amountMinor: 100000, currency: "RUB" },
        paidOn: "2026-03-10",
      }),
    );
    expect(await screen.findByText("Partially paid")).toBeInTheDocument();
  });

  it("saves the PDF where the user chooses", async () => {
    const user = setup();
    await actions(user, "INV-2026-001", "Save PDF…");
    await waitFor(() =>
      expect(calls.find((c) => c.cmd === "save_invoice_pdf")?.args).toEqual({
        id: "i1",
        path: "/home/me/INV-2026-001.pdf",
      }),
    );
    expect(await screen.findByText("PDF saved")).toBeInTheDocument();
  });

  it("previews the PDF in the system viewer", async () => {
    const user = setup();
    await actions(user, "INV-2026-001", "Preview PDF");
    await waitFor(() => expect(calls.some((c) => c.cmd === "open_invoice_pdf")).toBe(true));
  });

  it("opens the editor and the from-time wizard", async () => {
    const user = setup();
    await user.click(await screen.findByText("INV-2026-001"));
    expect(screen.getByRole("dialog", { name: "Invoice INV-2026-001" })).toBeInTheDocument();
    await user.click(screen.getAllByRole("button", { name: "Cancel" })[0]);
    await user.click(screen.getByRole("button", { name: "From time" }));
    expect(screen.getByRole("dialog", { name: "Invoice from tracked time" })).toBeInTheDocument();
  });

  it("deletes after confirmation", async () => {
    const user = setup();
    await actions(user, "INV-2026-001", "Delete");
    await user.click(
      within(screen.getByRole("alertdialog")).getByRole("button", { name: "Delete" }),
    );
    await waitFor(() =>
      expect(calls.find((c) => c.cmd === "delete_invoice")?.args).toEqual({ id: "i1" }),
    );
  });

  it("has no a11y violations", async () => {
    setup();
    await screen.findByText("INV-2026-001");
    await expectNoA11yViolations(document.body);
  });
});
