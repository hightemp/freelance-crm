import { render, screen, waitFor } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, describe, expect, it, vi } from "vitest";
import { COLUMNS, task } from "./fixtures";
import TaskEdit from "./TaskEdit.svelte";

afterEach(() => clearMocks());

describe("TaskEdit", () => {
  it("loads the task into the form and saves it", async () => {
    mockIPC((cmd) => {
      if (cmd === "get_task" || cmd === "update_task") return task("t1", "Макет");
      if (cmd === "list_board_columns") return COLUMNS;
      return [];
    });
    const onsaved = vi.fn();
    render(TaskEdit, { id: "t1", onsaved, oncancel: vi.fn() });
    expect(await screen.findByRole("textbox", { name: "Title" })).toHaveValue("Макет");
    await userEvent.click(screen.getByRole("button", { name: "Save" }));
    await waitFor(() => expect(onsaved).toHaveBeenCalledOnce());
  });

  it("shows a load error", async () => {
    mockIPC(() => {
      throw { code: "task.not_found", params: { id: "t1" } };
    });
    render(TaskEdit, { id: "t1", onsaved: vi.fn(), oncancel: vi.fn() });
    expect(await screen.findByRole("alert")).toHaveTextContent("Task not found");
  });
});
