import { cleanup, render, screen, waitFor, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { TimeEntry_Serialize as TimeEntry } from "$lib/api/bindings";
import { expectNoA11yViolations } from "../../../test/a11y";
import { CLIENTS, invoice, PROJECTS } from "./fixtures";
import FromTimeDialog from "./FromTimeDialog.svelte";

const at = (s: string) => Date.parse(s);

function entry(id: string, from: string, to: string, description: string): TimeEntry {
  return {
    id,
    taskId: null,
    taskTitle: null,
    projectId: "p1",
    projectName: "Сайт",
    clientId: "c1",
    clientName: "Ромашка",
    description,
    startedAt: at(from),
    endedAt: at(to),
    billable: true,
    invoiceId: null,
    createdAt: 1,
    updatedAt: 1,
  };
}

let calls: { cmd: string; args: Record<string, unknown> }[] = [];
let unbilled: TimeEntry[] = [];

beforeEach(() => {
  calls = [];
  unbilled = [
    entry("e1", "2026-03-02T09:00:00Z", "2026-03-02T10:30:00Z", "Вёрстка"),
    entry("e2", "2026-03-03T09:00:00Z", "2026-03-03T10:00:00Z", "Правки"),
  ];
  mockIPC((cmd, args) => {
    const a = (args ?? {}) as Record<string, unknown>;
    calls.push({ cmd, args: a });
    switch (cmd) {
      case "unbilled_time":
        return unbilled;
      case "invoice_from_time":
        return invoice({ id: "draft", storedStatus: "draft", status: "draft" });
    }
  });
});

afterEach(() => {
  cleanup();
  clearMocks();
});

function setup() {
  const oncreated = vi.fn();
  render(FromTimeDialog, {
    clients: CLIENTS,
    projects: PROJECTS,
    today: "2026-03-10",
    oncreated,
    onclose: () => {},
  });
  return { oncreated, user: userEvent.setup() };
}

async function pickClient(user: ReturnType<typeof userEvent.setup>) {
  await user.click(screen.getByRole("combobox", { name: /Client/ }));
  await user.click(screen.getByRole("option", { name: "Ромашка" }));
}

describe("FromTimeDialog", () => {
  it("shows the client's unbilled entries and their total", async () => {
    const { user } = setup();
    expect(screen.getByRole("button", { name: "Create draft" })).toBeDisabled();
    await pickClient(user);
    const list = await screen.findByRole("list", { name: "Unbilled time" });
    const rows = within(list).getAllByRole("listitem");
    expect(rows.map((r) => r.textContent)).toEqual([
      expect.stringContaining("Вёрстка"),
      expect.stringContaining("Правки"),
    ]);
    expect(within(rows[0]).getByText("1:30")).toBeInTheDocument();
    expect(screen.getByText("Total: 2:30")).toBeInTheDocument();

    const query = calls.find((c) => c.cmd === "unbilled_time");
    expect(query?.args.filter).toMatchObject({
      clientId: "c1",
      projectId: null,
      from: at("2026-03-01T00:00:00Z"),
      to: at("2026-03-11T00:00:00Z"),
    });
  });

  it("creates a draft from the period", async () => {
    const { user, oncreated } = setup();
    await pickClient(user);
    await screen.findByText("Total: 2:30");
    await user.click(screen.getByRole("radio", { name: "One line" }));
    await user.type(screen.getByRole("textbox", { name: "Line description" }), "Работы за март");
    await user.click(screen.getByRole("button", { name: "Create draft" }));
    await waitFor(() => expect(oncreated).toHaveBeenCalled());
    expect(calls.find((c) => c.cmd === "invoice_from_time")?.args.input).toMatchObject({
      clientId: "c1",
      grouping: "single_line",
      description: "Работы за март",
      number: "",
      issueDate: "2026-03-10",
    });
  });

  it("says when there is nothing to bill", async () => {
    unbilled = [];
    const { user } = setup();
    await pickClient(user);
    expect(await screen.findByText(/No unbilled billable time/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Create draft" })).toBeDisabled();
  });

  it("has no a11y violations", async () => {
    const { user } = setup();
    await pickClient(user);
    await screen.findByText("Total: 2:30");
    await expectNoA11yViolations(document.body);
  });
});
