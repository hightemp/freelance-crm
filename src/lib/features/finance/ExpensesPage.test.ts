import { cleanup, render, screen, waitFor, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import type { Expense_Serialize as Expense } from "$lib/api/bindings";
import { appSettings, DEFAULT_SETTINGS } from "$lib/app/settings.svelte";
import DialogHost from "$lib/ui/DialogHost.svelte";
import { resetDialogs } from "$lib/ui/dialogs.svelte";
import { expectNoA11yViolations } from "../../../test/a11y";
import ExpensesPage from "./ExpensesPage.svelte";
import { PROJECTS } from "./fixtures";

let calls: { cmd: string; args: Record<string, unknown> }[] = [];
let expenses: Expense[] = [];

const expense = (id: string, patch: Partial<Expense> = {}): Expense => ({
  id,
  projectId: null,
  projectName: null,
  category: "Софт",
  amount: { amountMinor: 99000, currency: "RUB" },
  spentOn: "2026-03-05",
  description: "JetBrains",
  createdAt: 1,
  updatedAt: 1,
  ...patch,
});

beforeEach(() => {
  calls = [];
  resetDialogs();
  appSettings.value = { ...DEFAULT_SETTINGS, defaultCurrency: "RUB" };
  expenses = [
    expense("x1"),
    expense("x2", {
      category: "Хостинг",
      amount: { amountMinor: 500, currency: "USD" },
      projectId: "p1",
      projectName: "Сайт",
      description: "VPS",
    }),
  ];
  mockIPC((cmd, args) => {
    const a = (args ?? {}) as Record<string, unknown>;
    calls.push({ cmd, args: a });
    switch (cmd) {
      case "list_expenses":
        return expenses;
      case "expense_categories":
        return ["Софт", "Хостинг"];
      case "list_projects":
        return PROJECTS;
      case "create_expense":
        return expense("x3");
      case "delete_expense":
        expenses = expenses.filter((e) => e.id !== a.id);
        return null;
      case "list_attachments":
        return [];
    }
  });
});

afterEach(() => {
  cleanup();
  clearMocks();
});

function setup() {
  render(ExpensesPage, { today: "2026-03-10" });
  render(DialogHost);
  return userEvent.setup();
}

describe("ExpensesPage", () => {
  it("lists expenses of the month with totals per currency", async () => {
    setup();
    const row = (await screen.findByText("VPS")).closest("tr")!;
    expect(within(row).getByText("Хостинг")).toBeInTheDocument();
    expect(within(row).getByText("Сайт")).toBeInTheDocument();
    expect(within(row).getByText("$5.00")).toBeInTheDocument();
    expect(screen.getByText("Total: RUB 990.00 · $5.00")).toBeInTheDocument();
    expect(calls.find((c) => c.cmd === "list_expenses")?.args.filter).toEqual({
      projectId: null,
      from: "2026-03-01",
      to: "2026-03-10",
      category: null,
    });
  });

  it("filters by category", async () => {
    const user = setup();
    await screen.findByText("VPS");
    await user.click(screen.getByRole("combobox", { name: "Category" }));
    await user.click(screen.getByRole("option", { name: "Хостинг" }));
    await waitFor(() =>
      expect(calls.filter((c) => c.cmd === "list_expenses").at(-1)?.args.filter).toMatchObject({
        category: "Хостинг",
      }),
    );
  });

  it("adds an expense with a new category", async () => {
    const user = setup();
    await screen.findByText("VPS");
    await user.click(screen.getByRole("button", { name: "Add expense" }));
    const dialog = screen.getByRole("dialog", { name: "New expense" });
    await user.type(within(dialog).getByRole("textbox", { name: "Amount" }), "1500");
    await user.type(within(dialog).getByRole("combobox", { name: /Category/ }), "Обучение");
    await user.click(screen.getByRole("option", { name: "Create “Обучение”" }));
    await user.type(within(dialog).getByRole("textbox", { name: "Description" }), "Курс");
    await user.click(within(dialog).getByRole("button", { name: "Save" }));
    await waitFor(() =>
      expect(calls.find((c) => c.cmd === "create_expense")?.args.input).toEqual({
        projectId: null,
        category: "Обучение",
        amount: { amountMinor: 150000, currency: "RUB" },
        spentOn: "2026-03-10",
        description: "Курс",
      }),
    );
  });

  it("deletes after confirmation", async () => {
    const user = setup();
    await screen.findByText("VPS");
    await user.click(screen.getByRole("button", { name: "Delete expense VPS" }));
    await user.click(
      within(screen.getByRole("alertdialog")).getByRole("button", { name: "Delete" }),
    );
    await waitFor(() => expect(screen.queryByText("VPS")).toBeNull());
  });

  it("has no a11y violations", async () => {
    setup();
    await screen.findByText("VPS");
    await expectNoA11yViolations(document.body);
  });
});
