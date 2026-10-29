import { render, screen, waitFor, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Task } from "$lib/api/bindings";
import DialogHost from "$lib/ui/DialogHost.svelte";
import { resetDialogs } from "$lib/ui/dialogs.svelte";
import { expectNoA11yViolations } from "../../../test/a11y";
import { COLUMNS, task } from "./fixtures";
import TaskCard from "./TaskCard.svelte";

const goto = vi.hoisted(() => vi.fn());
vi.mock("$app/navigation", () => ({ goto }));

let current: Task;
let calls: { cmd: string; args: Record<string, unknown> }[] = [];

beforeEach(() => {
  calls = [];
  current = task("t1", "Макет главной", {
    priority: "high",
    clientId: "c1",
    clientName: "Ромашка",
    projectId: "p1",
    projectName: "Сайт",
    dueDate: "2024-05-01",
    dueTime: "18:00",
    estimateMinutes: 90,
    descriptionMd: "Нужно **до пятницы**",
  });
  mockIPC((cmd, args) => {
    const a = (args ?? {}) as Record<string, unknown>;
    calls.push({ cmd, args: a });
    switch (cmd) {
      case "get_task":
        if (a.id !== current.id) throw { code: "task.not_found", params: { id: a.id } };
        return current;
      case "list_board_columns":
        return COLUMNS;
      case "set_task_status":
        current = { ...current, statusId: a.statusId as string, completedAt: 5 };
        return current;
      case "delete_task":
        return null;
      case "list_tags":
      case "list_checklist":
      case "list_notes":
        return [];
    }
  });
});

afterEach(() => {
  clearMocks();
  resetDialogs();
  goto.mockReset();
});

describe("TaskCard", () => {
  it("shows the task with its links, priority, deadline and description", async () => {
    render(TaskCard, { id: "t1" });
    expect(await screen.findByRole("heading", { level: 1, name: "Макет главной" })).toBeVisible();
    expect(screen.getByRole("link", { name: "Ромашка" })).toHaveAttribute("href", "/clients/c1");
    expect(screen.getByRole("link", { name: "Сайт" })).toHaveAttribute("href", "/projects/p1");
    expect(screen.getByText("High")).toBeInTheDocument();
    expect(screen.getByText("May 1, 2024, 18:00")).toBeInTheDocument();
    expect(screen.getByText("1.5 h")).toBeInTheDocument();
    expect(screen.getByText("до пятницы").tagName).toBe("STRONG");
    expect(await screen.findByRole("heading", { name: "Checklist" })).toBeInTheDocument();
    expect(screen.getByRole("combobox", { name: "Tags" })).toBeInTheDocument();
  });

  it("moves the task to another column", async () => {
    render(TaskCard, { id: "t1" });
    await waitFor(() =>
      expect(screen.getByRole("combobox", { name: "Column" })).toHaveTextContent("To do"),
    );
    await userEvent.click(screen.getByRole("combobox", { name: "Column" }));
    await userEvent.click(screen.getByRole("option", { name: "Done" }));
    await waitFor(() => expect(screen.getByText("Completed")).toBeInTheDocument());
    expect(calls.find((c) => c.cmd === "set_task_status")!.args).toEqual({
      id: "t1",
      statusId: "s-done",
    });
  });

  it("opens the edit page and deletes after confirmation", async () => {
    render(DialogHost);
    render(TaskCard, { id: "t1" });
    await userEvent.click(await screen.findByRole("button", { name: "Edit" }));
    expect(goto).toHaveBeenCalledWith("/tasks/t1/edit");

    await userEvent.click(screen.getByRole("button", { name: "More actions" }));
    await userEvent.click(screen.getByRole("menuitem", { name: "Delete task" }));
    const dialog = await screen.findByRole("alertdialog", { name: "Delete task Макет главной?" });
    await userEvent.click(within(dialog).getByRole("button", { name: "Delete" }));
    await waitFor(() => expect(goto).toHaveBeenCalledWith("/tasks"));
  });

  it("shows inbox tasks without links", async () => {
    current = task("t1", "Позвонить");
    render(TaskCard, { id: "t1" });
    expect(await screen.findByText("Inbox")).toBeInTheDocument();
    expect(screen.queryByRole("link")).not.toBeInTheDocument();
  });

  it("shows an error for an unknown task", async () => {
    render(TaskCard, { id: "missing" });
    expect(await screen.findByRole("alert")).toHaveTextContent("Task not found");
  });

  it("has no a11y violations", async () => {
    const { container } = render(TaskCard, { id: "t1" });
    await screen.findByRole("heading", { name: "Checklist" });
    await expectNoA11yViolations(container);
  });
});
