import { render, screen, waitFor } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { TaskInput_Deserialize as TaskInput } from "$lib/api/bindings";
import { setAppLocale } from "$lib/i18n/locale.svelte";
import { expectNoA11yViolations } from "../../../test/a11y";
import { editorText } from "../../../test/editor";
import { client, project } from "../projects/fixtures";
import { COLUMNS, task } from "./fixtures";
import TaskForm from "./TaskForm.svelte";

let calls: { cmd: string; args: Record<string, unknown> }[] = [];

beforeEach(() => {
  calls = [];
  mockIPC((cmd, args) => {
    const a = (args ?? {}) as Record<string, unknown>;
    calls.push({ cmd, args: a });
    switch (cmd) {
      case "list_board_columns":
        return COLUMNS;
      case "list_clients":
        return [client("c1", "Ромашка"), client("c2", "Василёк")];
      case "list_projects":
        return [
          project("p1", "Сайт", { clientId: "c1" }),
          project("p2", "Лендинг", { clientId: "c2", clientName: "Василёк" }),
        ];
      case "create_task":
      case "update_task":
        return task((a.id as string) ?? "t-new", (a.input as TaskInput).title);
    }
  });
});

afterEach(() => {
  clearMocks();
  setAppLocale("en");
});

const box = (name: string) => screen.getByRole("textbox", { name });
const combo = (name: string) => screen.getByRole("combobox", { name });
const optionNames = () => screen.getAllByRole("option").map((o) => o.textContent?.trim());
const sent = (cmd: string) => calls.find((c) => c.cmd === cmd)!.args;

async function choose(label: string, option: string) {
  await userEvent.click(combo(label));
  await userEvent.click(screen.getByRole("option", { name: option }));
}

async function ready() {
  await waitFor(() => expect(combo("Column")).toHaveTextContent("To do"));
}

describe("TaskForm", () => {
  it("creates an inbox task in the first to-do column", async () => {
    const onsaved = vi.fn();
    render(TaskForm, { onsaved });
    await ready();
    await userEvent.type(box("Title"), "Позвонить бухгалтеру");
    await userEvent.click(screen.getByRole("button", { name: "Save" }));
    await waitFor(() => expect(onsaved).toHaveBeenCalledOnce());
    expect(sent("create_task").input).toEqual({
      title: "Позвонить бухгалтеру",
      descriptionMd: "",
      statusId: "s-todo",
      priority: "normal",
      projectId: null,
      clientId: null,
      dueDate: null,
      dueTime: null,
      estimateMinutes: null,
    });
  });

  it("choosing a project sets its client and narrows the clients", async () => {
    render(TaskForm, { onsaved: vi.fn() });
    await ready();
    await choose("Project", "Лендинг");
    expect(combo("Client")).toHaveTextContent("Василёк");
    await userEvent.click(combo("Client"));
    expect(optionNames()).toEqual(["Василёк"]);
    await userEvent.keyboard("{Escape}");

    await choose("Project", "No project");
    await userEvent.click(combo("Client"));
    expect(optionNames()).toEqual(["No client (inbox)", "Ромашка", "Василёк"]);
  });

  it("choosing a client narrows the projects", async () => {
    render(TaskForm, { onsaved: vi.fn() });
    await ready();
    await choose("Client", "Василёк");
    await userEvent.click(combo("Project"));
    expect(optionNames()).toEqual(["No project", "Лендинг"]);
  });

  it("starts with the given project or client", async () => {
    render(TaskForm, { projectId: "p1", onsaved: vi.fn() });
    await waitFor(() => expect(combo("Project")).toHaveTextContent("Сайт"));
    expect(combo("Client")).toHaveTextContent("Ромашка");
  });

  it("validates the title and the time", async () => {
    render(TaskForm, { onsaved: vi.fn() });
    await ready();
    await userEvent.type(box("Time"), "10:00");
    await userEvent.click(screen.getByRole("button", { name: "Save" }));
    expect(box("Title")).toHaveAccessibleDescription("This field is required");
    expect(box("Time")).toHaveAccessibleDescription("Set a date first");
    expect(calls.some((c) => c.cmd === "create_task")).toBe(false);

    setAppLocale("ru");
    await waitFor(() => expect(box("Название")).toHaveAccessibleDescription("Обязательное поле"));
  });

  it("edits an existing task, converting the estimate from hours", async () => {
    const onsaved = vi.fn();
    render(TaskForm, {
      task: task("t1", "Макет", {
        descriptionMd: "## Шаги",
        priority: "high",
        estimateMinutes: 90,
        projectId: "p1",
        clientId: "c1",
      }),
      onsaved,
    });
    await waitFor(() => expect(box("Title")).toHaveValue("Макет"));
    expect(box("Estimate, hours")).toHaveValue("1.5");
    expect(editorText(screen.getByRole("textbox", { name: "Description" }))).toBe("## Шаги");
    await userEvent.clear(box("Estimate, hours"));
    await userEvent.type(box("Estimate, hours"), "2");
    await choose("Priority", "Urgent");
    await userEvent.click(screen.getByRole("button", { name: "Save" }));
    await waitFor(() => expect(onsaved).toHaveBeenCalledOnce());
    expect(sent("update_task")).toMatchObject({
      id: "t1",
      input: { priority: "urgent", estimateMinutes: 120, projectId: "p1", clientId: "c1" },
    });
  });

  it("has no a11y violations", async () => {
    const { container } = render(TaskForm, { onsaved: vi.fn() });
    await ready();
    await expectNoA11yViolations(container);
  });
});
