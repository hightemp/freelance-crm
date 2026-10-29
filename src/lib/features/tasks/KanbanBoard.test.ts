import { fireEvent, render, screen, waitFor, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import type { BoardColumn, Task } from "$lib/api/bindings";
import { setAppLocale } from "$lib/i18n/locale.svelte";
import { expectNoA11yViolations } from "../../../test/a11y";
import { COLUMNS, task } from "./fixtures";
import KanbanBoard from "./KanbanBoard.svelte";

let columns: BoardColumn[] = [];
let tasks: Task[] = [];
let calls: { cmd: string; args: Record<string, unknown> }[] = [];
let failMove = false;

beforeEach(() => {
  failMove = false;
  calls = [];
  columns = COLUMNS.map((c) => ({ ...c }));
  tasks = [
    task("a", "Макет", { boardPosition: "V", checklistDone: 1, checklistTotal: 2 }),
    task("b", "Вёрстка", { boardPosition: "W", priority: "urgent", clientName: "Ромашка" }),
    task("c", "Ревью", { statusId: "s-doing" }),
  ];
  mockIPC((cmd, args) => {
    const a = (args ?? {}) as Record<string, unknown>;
    calls.push({ cmd, args: a });
    switch (cmd) {
      case "list_board_columns":
        return columns.map((c) => ({
          ...c,
          taskCount: tasks.filter((t) => t.statusId === c.id).length,
        }));
      case "list_tasks":
        return tasks;
      case "move_task":
        if (failMove) throw { code: "task.stale_position", params: {} };
        tasks = tasks.map((t) => (t.id === a.id ? { ...t, statusId: a.statusId as string } : t));
        return tasks.find((t) => t.id === a.id);
      case "create_status": {
        const input = a.input as { name: string; category: BoardColumn["category"] };
        const created = { ...COLUMNS[1], id: "s-new", nameKey: null, ...input, position: 9 };
        columns.push({ ...created, taskCount: 0, overLimit: false });
        return created;
      }
      case "update_status": {
        const i = columns.findIndex((c) => c.id === a.id);
        columns[i] = { ...columns[i], ...(a.input as object) };
        return columns[i];
      }
      case "reorder_statuses":
        return [];
      case "delete_status":
        tasks = tasks.map((t) =>
          t.statusId === a.id ? { ...t, statusId: a.moveTo as string } : t,
        );
        columns = columns.filter((c) => c.id !== a.id);
        return null;
    }
  });
});

afterEach(() => {
  clearMocks();
  setAppLocale("en");
});

const zone = (name: string) => screen.getByRole("list", { name: `Tasks in ${name}` });
const cardTitles = (name: string) =>
  within(zone(name))
    .queryAllByRole("listitem")
    .map((card) => within(card).getByRole("heading").textContent?.trim());
const commands = () => calls.map((c) => c.cmd);

/** What svelte-dnd-action emits when a card is dropped into a zone. */
async function drop(name: string, order: Task[], id: string) {
  await fireEvent(
    zone(name),
    new CustomEvent("finalize", {
      detail: { items: order, info: { id, trigger: "droppedIntoZone", source: "pointer" } },
    }),
  );
}

async function columnMenu(name: string, item: string) {
  await userEvent.click(screen.getByRole("button", { name: `Actions for column ${name}` }));
  await userEvent.click(screen.getByRole("menuitem", { name: item }));
}

describe("KanbanBoard", () => {
  it("shows columns with localized names, counts and cards", async () => {
    render(KanbanBoard);
    await waitFor(() => expect(cardTitles("To do")).toEqual(["Макет", "Вёрстка"]));
    expect(cardTitles("In progress")).toEqual(["Ревью"]);
    expect(cardTitles("Done")).toEqual([]);
    const card = within(zone("To do")).getAllByRole("listitem")[1];
    expect(within(card).getByText("Urgent")).toBeInTheDocument();
    expect(within(card).getByText("Ромашка")).toBeInTheDocument();
    expect(screen.getByRole("region", { name: "To do" })).toHaveTextContent("2");

    setAppLocale("ru");
    await waitFor(() => expect(screen.getByRole("region", { name: "На проверке" })).toBeVisible());
  });

  it("moves a dropped card between its new neighbours", async () => {
    render(KanbanBoard);
    await waitFor(() => expect(cardTitles("In progress")).toEqual(["Ревью"]));
    const [a] = tasks;
    await drop("In progress", [tasks[2], a], "a");
    await waitFor(() => expect(commands()).toContain("move_task"));
    expect(calls.find((c) => c.cmd === "move_task")!.args).toEqual({
      id: "a",
      statusId: "s-doing",
      after: "c",
      before: null,
    });
    expect(cardTitles("In progress")).toEqual(["Ревью", "Макет"]);
  });

  it("rolls back when the move fails", async () => {
    failMove = true;
    render(KanbanBoard);
    await waitFor(() => expect(cardTitles("In progress")).toEqual(["Ревью"]));
    await drop("In progress", [tasks[0], tasks[2]], "a");
    await waitFor(() => expect(cardTitles("In progress")).toEqual(["Ревью"]));
    expect(cardTitles("To do")).toEqual(["Макет", "Вёрстка"]);
  });

  it("adds and renames a column on the board", async () => {
    render(KanbanBoard);
    await screen.findByRole("region", { name: "To do" });
    await userEvent.click(screen.getByRole("button", { name: "Add column" }));
    const dialog = await screen.findByRole("dialog", { name: "New column" });
    await userEvent.type(within(dialog).getByRole("textbox", { name: "Name" }), "Тесты");
    await userEvent.click(within(dialog).getByRole("button", { name: "Save" }));
    expect(await screen.findByRole("region", { name: "Тесты" })).toBeInTheDocument();
    expect(calls.find((c) => c.cmd === "create_status")!.args.input).toMatchObject({
      name: "Тесты",
      category: "in_progress",
      wipLimit: null,
    });

    await columnMenu("Review", "Edit column");
    const edit = await screen.findByRole("dialog", { name: "Edit column" });
    const name = within(edit).getByRole("textbox", { name: "Name" });
    expect(name).toHaveValue("Review");
    await userEvent.clear(name);
    await userEvent.type(name, "QA");
    await userEvent.type(within(edit).getByRole("textbox", { name: "WIP limit" }), "3");
    await userEvent.click(within(edit).getByRole("button", { name: "Save" }));
    expect(await screen.findByRole("region", { name: "QA" })).toBeInTheDocument();
    expect(calls.find((c) => c.cmd === "update_status")!.args).toMatchObject({
      id: "s-review",
      input: { name: "QA", wipLimit: 3 },
    });
  });

  it("keeps a default column localized when its name is not changed", async () => {
    render(KanbanBoard);
    await screen.findByRole("region", { name: "To do" });
    await columnMenu("To do", "Edit column");
    const edit = await screen.findByRole("dialog", { name: "Edit column" });
    await userEvent.click(within(edit).getByRole("button", { name: "Save" }));
    await waitFor(() => expect(commands()).toContain("update_status"));
    expect(calls.find((c) => c.cmd === "update_status")!.args.input).toMatchObject({ name: null });
  });

  it("reorders columns when one is dragged", async () => {
    render(KanbanBoard);
    await screen.findByRole("region", { name: "To do" });
    const strip = screen.getByRole("list", { name: "Board" });
    const reversed = [...columns].reverse();
    await fireEvent(
      strip,
      new CustomEvent("finalize", {
        detail: {
          items: reversed,
          info: { id: "s-done", trigger: "droppedIntoZone", source: "pointer" },
        },
      }),
    );
    await waitFor(() => expect(commands()).toContain("reorder_statuses"));
    expect(calls.find((c) => c.cmd === "reorder_statuses")!.args.ids).toEqual(
      reversed.map((c) => c.id),
    );
  });

  it("deletes a column after choosing where its tasks go", async () => {
    render(KanbanBoard);
    await screen.findByRole("region", { name: "In progress" });
    await columnMenu("In progress", "Delete column");
    const dialog = await screen.findByRole("alertdialog", { name: "Delete column In progress?" });
    expect(within(dialog).getByRole("button", { name: "Delete" })).toBeDisabled();
    await userEvent.click(within(dialog).getByRole("combobox", { name: "Move its 1 tasks to" }));
    await userEvent.click(screen.getByRole("option", { name: "Review" }));
    await userEvent.click(within(dialog).getByRole("button", { name: "Delete" }));
    await waitFor(() =>
      expect(screen.queryByRole("region", { name: "In progress" })).not.toBeInTheDocument(),
    );
    expect(calls.find((c) => c.cmd === "delete_status")!.args).toEqual({
      id: "s-doing",
      moveTo: "s-review",
    });
    expect(cardTitles("Review")).toEqual(["Ревью"]);
  });

  it("warns when a column is over its WIP limit", async () => {
    columns[0] = { ...columns[0], wipLimit: 1 };
    render(KanbanBoard);
    const todo = await screen.findByRole("region", { name: "To do" });
    await waitFor(() => expect(within(todo).getByText("2 of 1")).toBeInTheDocument());
    expect(within(todo).getByText("Over the WIP limit")).toBeInTheDocument();
  });

  it("has no a11y violations", async () => {
    const { container } = render(KanbanBoard);
    await waitFor(() => expect(cardTitles("To do")).toHaveLength(2));
    await expectNoA11yViolations(container);
  });
});
