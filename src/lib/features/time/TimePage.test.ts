import { cleanup, render, screen, waitFor, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import type { TimeEntry_Serialize as TimeEntry } from "$lib/api/bindings";
import DialogHost from "$lib/ui/DialogHost.svelte";
import { resetDialogs } from "$lib/ui/dialogs.svelte";
import ToastHost from "$lib/ui/ToastHost.svelte";
import { expectNoA11yViolations } from "../../../test/a11y";
import TimePage from "./TimePage.svelte";

const at = (s: string) => Date.parse(s);

function entry(id: string, from: string, to: string, patch: Partial<TimeEntry> = {}): TimeEntry {
  return {
    id,
    taskId: null,
    taskTitle: null,
    projectId: "p1",
    projectName: "Сайт",
    clientId: "c1",
    clientName: "Ромашка",
    description: "",
    startedAt: at(from),
    endedAt: at(to),
    billable: true,
    invoiceId: null,
    createdAt: 1,
    updatedAt: 1,
    ...patch,
  };
}

const ENTRIES = [
  entry("e2", "2026-03-11T10:00:00Z", "2026-03-11T11:30:00Z", { description: "Правки" }),
  entry("e1", "2026-03-10T09:00:00Z", "2026-03-10T10:00:00Z", {
    description: "Вёрстка",
    taskId: "k1",
    taskTitle: "Макет",
  }),
  entry("e0", "2026-03-02T09:00:00Z", "2026-03-02T09:30:00Z", {
    description: "Старое",
    invoiceId: "i1",
  }),
];

let calls: { cmd: string; args: Record<string, unknown> }[] = [];

beforeEach(() => {
  calls = [];
  mockIPC((cmd, args) => {
    const a = (args ?? {}) as Record<string, unknown>;
    calls.push({ cmd, args: a });
    switch (cmd) {
      case "list_time_entries":
        return ENTRIES;
      case "time_report": {
        const total = {
          key: "",
          label: "",
          seconds: 3 * 3600,
          billableSeconds: 3 * 3600,
          amounts: [{ amountMinor: 900_000, currency: "RUB" }],
        };
        const rows =
          a.group === "project"
            ? [{ ...total, key: "p1", label: "Сайт" }]
            : [{ ...total, key: "2026-03-10" }];
        return { rows, total };
      }
      case "update_time_entry":
        return { entry: ENTRIES[1], overlaps: ["e2"] };
      case "create_time_entry":
        return { entry: ENTRIES[0], overlaps: [] };
      case "delete_time_entry":
      case "export_time_csv":
        return null;
      case "plugin:dialog|save":
        return "/home/me/time.csv";
      case "list_clients":
        return [{ id: "c1", displayName: "Ромашка" }];
      case "list_projects":
        return [{ id: "p1", name: "Сайт", clientId: "c1", clientName: "Ромашка" }];
      case "list_tasks":
        return [];
    }
  });
});

afterEach(() => {
  cleanup();
  clearMocks();
  resetDialogs();
});

const lastFilter = (cmd: string) =>
  calls.findLast((c) => c.cmd === cmd)!.args.filter as Record<string, unknown>;

function show() {
  render(DialogHost);
  render(ToastHost);
  return render(TimePage, { today: "2026-03-18" });
}

describe("TimePage", () => {
  it("lists the month's entries with totals", async () => {
    show();
    const list = await screen.findByRole("list", { name: "Time entries" });
    expect(within(list).getAllByRole("listitem")).toHaveLength(3);
    expect(within(list).getByText("Макет · Вёрстка")).toBeInTheDocument();
    expect(screen.getByText("Total: 3:00")).toBeInTheDocument();
    expect(screen.getByText("RUB 9,000.00")).toBeInTheDocument();
    expect(lastFilter("list_time_entries")).toEqual({
      from: new Date(2026, 2, 1).getTime(),
      to: new Date(2026, 2, 19).getTime(),
      clientId: null,
      projectId: null,
      billable: null,
    });
  });

  it("filters by client and billable flag", async () => {
    show();
    await screen.findByRole("list", { name: "Time entries" });
    await userEvent.click(screen.getByRole("combobox", { name: "Client" }));
    await userEvent.click(await screen.findByRole("option", { name: "Ромашка" }));
    await userEvent.click(screen.getByRole("combobox", { name: "Billable" }));
    await userEvent.click(await screen.findByRole("option", { name: "Not billable" }));
    await waitFor(() =>
      expect(lastFilter("list_time_entries")).toMatchObject({ clientId: "c1", billable: false }),
    );
    expect(lastFilter("time_report")).toMatchObject({ clientId: "c1", billable: false });
  });

  it("edits an entry in place and warns about overlaps", async () => {
    show();
    const list = await screen.findByRole("list", { name: "Time entries" });
    await userEvent.click(
      within(list).getByRole("button", { name: "Edit entry: Макет · Вёрстка" }),
    );
    const description = within(list).getByLabelText("Description");
    await userEvent.clear(description);
    await userEvent.type(description, "Вёрстка шапки");
    await userEvent.click(within(list).getByRole("button", { name: "Save" }));
    await waitFor(() => expect(calls.some((c) => c.cmd === "update_time_entry")).toBe(true));
    expect(calls.find((c) => c.cmd === "update_time_entry")!.args).toEqual({
      id: "e1",
      input: {
        target: { taskId: "k1", projectId: "p1", clientId: "c1" },
        description: "Вёрстка шапки",
        startedAt: at("2026-03-10T09:00:00Z"),
        endedAt: at("2026-03-10T10:00:00Z"),
        billable: true,
      },
    });
    expect(await screen.findByText("Overlaps 1 other entry")).toBeInTheDocument();
  });

  it("locks invoiced entries", async () => {
    show();
    const list = await screen.findByRole("list", { name: "Time entries" });
    const old = within(list).getByText("Сайт · Старое").closest("li")!;
    expect(within(old).getByText("Invoiced")).toBeInTheDocument();
    expect(within(old).queryByRole("button")).not.toBeInTheDocument();
  });

  it("deletes after confirmation", async () => {
    show();
    const list = await screen.findByRole("list", { name: "Time entries" });
    await userEvent.click(
      within(list).getByRole("button", { name: "Delete entry: Сайт · Правки" }),
    );
    const dialog = await screen.findByRole("alertdialog");
    await userEvent.click(within(dialog).getByRole("button", { name: "Delete" }));
    await waitFor(() =>
      expect(calls.some((c) => c.cmd === "delete_time_entry" && c.args.id === "e2")).toBe(true),
    );
  });

  it("shows the report grouped as chosen", async () => {
    show();
    await userEvent.click(await screen.findByRole("tab", { name: "Report" }));
    await userEvent.click(screen.getByRole("combobox", { name: "Group by" }));
    await userEvent.click(await screen.findByRole("option", { name: "Project" }));
    const table = await screen.findByRole("table");
    expect(
      await within(table).findByRole("row", { name: /Сайт 3:00 RUB\s9,000\.00/ }),
    ).toBeInTheDocument();
    expect(calls.findLast((c) => c.cmd === "time_report")!.args.group).toBe("project");
  });

  it("exports CSV with translated headers", async () => {
    show();
    await screen.findByRole("list", { name: "Time entries" });
    await userEvent.click(screen.getByRole("button", { name: "Export CSV" }));
    await waitFor(() => expect(calls.some((c) => c.cmd === "export_time_csv")).toBe(true));
    const args = calls.find((c) => c.cmd === "export_time_csv")!.args;
    expect(args.path).toBe("/home/me/time.csv");
    expect(args.headers).toEqual([
      "Date",
      "Start",
      "End",
      "Hours",
      "Client",
      "Project",
      "Task",
      "Description",
      "Billable",
      "Amount",
      "Currency",
    ]);
  });

  it("adds an entry by hand", async () => {
    show();
    await screen.findByRole("list", { name: "Time entries" });
    await userEvent.click(screen.getByRole("button", { name: "Add entry" }));
    const dialog = await screen.findByRole("dialog", { name: "Add entry" });
    await userEvent.type(within(dialog).getByLabelText("Description"), "Созвон");
    await userEvent.click(within(dialog).getByRole("button", { name: "Save" }));
    await waitFor(() => expect(calls.some((c) => c.cmd === "create_time_entry")).toBe(true));
    expect(calls.find((c) => c.cmd === "create_time_entry")!.args.input).toMatchObject({
      description: "Созвон",
      startedAt: new Date(2026, 2, 18, 9, 0).getTime(),
      endedAt: new Date(2026, 2, 18, 10, 0).getTime(),
      billable: true,
    });
  });

  it("has no a11y violations", async () => {
    const { container } = show();
    await screen.findByRole("list", { name: "Time entries" });
    await expectNoA11yViolations(container);
  });
});
