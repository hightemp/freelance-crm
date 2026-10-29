import { fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import type { TaskInput_Deserialize as TaskInput } from "$lib/api/bindings";
import { expectNoA11yViolations } from "../../../test/a11y";
import { task } from "./fixtures";
import QuickTask from "./QuickTask.svelte";

let created: TaskInput[] = [];

beforeEach(() => {
  created = [];
  mockIPC((cmd, args) => {
    if (cmd === "create_task") {
      const input = (args as { input: TaskInput }).input;
      created.push(input);
      return task("t-new", input.title);
    }
  });
});

afterEach(() => clearMocks());

const shortcut = (init: KeyboardEventInit) => fireEvent.keyDown(window, { key: "n", ...init });

describe("QuickTask", () => {
  it("opens with Ctrl+N or Cmd+N", async () => {
    render(QuickTask);
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    await shortcut({ ctrlKey: true });
    expect(await screen.findByRole("dialog", { name: "Quick task" })).toBeInTheDocument();
    await userEvent.keyboard("{Escape}");
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
    await shortcut({ metaKey: true, key: "N" });
    expect(await screen.findByRole("dialog")).toBeInTheDocument();
  });

  it("ignores N without a modifier and with Shift or Alt", async () => {
    render(QuickTask);
    await shortcut({});
    await shortcut({ ctrlKey: true, altKey: true });
    await shortcut({ ctrlKey: true, shiftKey: true });
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  it("creates an inbox task with Enter", async () => {
    render(QuickTask);
    await shortcut({ ctrlKey: true });
    const title = await screen.findByRole("textbox", { name: "Title" });
    await waitFor(() => expect(title).toHaveFocus());
    await userEvent.type(title, "  Купить домен  {Enter}");
    await waitFor(() => expect(created).toHaveLength(1));
    expect(created[0]).toEqual({
      title: "Купить домен",
      descriptionMd: "",
      statusId: null,
      priority: "normal",
      projectId: null,
      clientId: null,
      dueDate: null,
      dueTime: null,
      estimateMinutes: null,
    });
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
  });

  it("does not create an empty task", async () => {
    render(QuickTask);
    await shortcut({ ctrlKey: true });
    await userEvent.type(await screen.findByRole("textbox", { name: "Title" }), "   {Enter}");
    expect(screen.getByRole("textbox", { name: "Title" })).toHaveAccessibleDescription(
      "This field is required",
    );
    expect(created).toEqual([]);
  });

  it("has no a11y violations", async () => {
    render(QuickTask);
    await shortcut({ ctrlKey: true });
    await expectNoA11yViolations((await screen.findByRole("dialog")).parentElement!);
  });
});
