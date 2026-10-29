import { render, screen, waitFor, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import type { TaskFilter_Deserialize as TaskFilter, Task } from "$lib/api/bindings";
import { setAppLocale } from "$lib/i18n/locale.svelte";
import { expectNoA11yViolations } from "../../../test/a11y";
import { COLUMNS, task } from "./fixtures";
import TaskList from "./TaskList.svelte";

let tasks: Task[] = [];
let requests: TaskFilter[] = [];
let statusCalls: { id: string; statusId: string }[] = [];

beforeEach(() => {
  requests = [];
  statusCalls = [];
  tasks = [
    task("t1", "Макет", {
      priority: "urgent",
      projectName: "Сайт",
      clientName: "Ромашка",
      dueDate: "2024-05-01",
      checklistDone: 1,
      checklistTotal: 3,
    }),
    task("t2", "Счёт", { statusId: "s-done", completedAt: 1 }),
  ];
  mockIPC((cmd, args) => {
    const a = args as Record<string, unknown>;
    switch (cmd) {
      case "list_tasks":
        requests.push(a.filter as TaskFilter);
        return tasks;
      case "list_board_columns":
        return COLUMNS;
      case "set_task_status":
        statusCalls.push({ id: a.id as string, statusId: a.statusId as string });
        tasks = tasks.map((t) => (t.id === a.id ? { ...t, statusId: a.statusId as string } : t));
        return tasks.find((t) => t.id === a.id);
    }
  });
});

afterEach(() => {
  clearMocks();
  setAppLocale("en");
});

const rows = () => within(screen.getByRole("list")).getAllByRole("listitem");
const lastRequest = () => requests.at(-1);

async function choose(label: string, option: string) {
  await userEvent.click(screen.getByRole("combobox", { name: label }));
  await userEvent.click(screen.getByRole("option", { name: option }));
}

describe("TaskList", () => {
  it("shows tasks with priority, column, links, deadline and checklist", async () => {
    render(TaskList);
    await waitFor(() => expect(rows()).toHaveLength(2));
    const [mockup, invoice] = rows();
    expect(within(mockup).getByText("Urgent")).toBeInTheDocument();
    expect(within(mockup).getByText("To do")).toBeInTheDocument();
    expect(within(mockup).getByText("Ромашка · Сайт")).toBeInTheDocument();
    expect(within(mockup).getByText("May 1, 2024")).toBeInTheDocument();
    expect(within(mockup).getByText("1/3")).toBeInTheDocument();
    expect(within(invoice).getByText("Inbox")).toBeInTheDocument();
    expect(within(invoice).getByRole("checkbox", { name: "Счёт" })).toBeChecked();
    expect(lastRequest()).toEqual({
      query: null,
      statusIds: [],
      priority: null,
      scope: null,
      tagIds: [],
      due: null,
      hideDone: false,
      sort: "board",
    });
  });

  it("updates the request when filters change", async () => {
    render(TaskList, { searchDelay: 0 });
    await waitFor(() => expect(rows()).toHaveLength(2));
    await choose("Column", "Review");
    await waitFor(() => expect(lastRequest()?.statusIds).toEqual(["s-review"]));
    await choose("Priority", "Urgent");
    await waitFor(() => expect(lastRequest()?.priority).toBe("urgent"));
    await choose("Deadline", "Overdue");
    await waitFor(() => expect(lastRequest()?.due).toBe("overdue"));
    await choose("Sort by", "Deadline");
    await waitFor(() => expect(lastRequest()?.sort).toBe("due"));
    await userEvent.click(screen.getByRole("switch", { name: "Hide completed" }));
    await waitFor(() => expect(lastRequest()?.hideDone).toBe(true));
    await userEvent.type(screen.getByRole("textbox", { name: "Search tasks" }), "мак");
    await waitFor(() => expect(lastRequest()?.query).toBe("мак"));
  });

  it("checks a task off into the first done column and back", async () => {
    render(TaskList);
    await waitFor(() => expect(rows()).toHaveLength(2));
    await userEvent.click(screen.getByRole("checkbox", { name: "Макет" }));
    await waitFor(() => expect(statusCalls).toEqual([{ id: "t1", statusId: "s-done" }]));
    await waitFor(() => expect(screen.getByRole("checkbox", { name: "Макет" })).toBeChecked());
    await userEvent.click(screen.getByRole("checkbox", { name: "Счёт" }));
    await waitFor(() => expect(statusCalls.at(-1)).toEqual({ id: "t2", statusId: "s-todo" }));
  });

  it("can be limited to a scope", async () => {
    render(TaskList, { scope: { type: "project", id: "p1" } });
    await waitFor(() => expect(lastRequest()?.scope).toEqual({ type: "project", id: "p1" }));
  });

  it("virtualizes 10 000 tasks", async () => {
    tasks = Array.from({ length: 10_000 }, (_, i) => task(`t${i}`, `Задача ${i}`));
    render(TaskList);
    await waitFor(() => expect(rows().length).toBeGreaterThan(0));
    expect(rows().length).toBeLessThan(40);
    expect(rows()[0]).toHaveAttribute("aria-setsize", "10000");
  });

  it("shows empty states", async () => {
    tasks = [];
    render(TaskList);
    expect(await screen.findByText("No tasks yet")).toBeInTheDocument();
    await choose("Deadline", "Today");
    expect(await screen.findByText("No tasks match the filters")).toBeInTheDocument();
  });

  it("names default columns in the current language", async () => {
    setAppLocale("ru");
    render(TaskList);
    await waitFor(() => expect(rows()).toHaveLength(2));
    expect(within(rows()[0]).getByText("К выполнению")).toBeInTheDocument();
  });

  it("has no a11y violations", async () => {
    const { container } = render(TaskList);
    await waitFor(() => expect(rows()).toHaveLength(2));
    await expectNoA11yViolations(container);
  });
});
