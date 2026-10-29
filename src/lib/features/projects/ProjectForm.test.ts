import { render, screen, waitFor } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { ProjectInput_Deserialize as ProjectInput } from "$lib/api/bindings";
import { setAppLocale } from "$lib/i18n/locale.svelte";
import { expectNoA11yViolations } from "../../../test/a11y";
import { editorText, typeInEditor } from "../../../test/editor";
import { client, project } from "./fixtures";
import ProjectForm from "./ProjectForm.svelte";

let calls: { cmd: string; args: Record<string, unknown> }[] = [];
let createError: unknown = null;

beforeEach(() => {
  calls = [];
  createError = null;
  mockIPC((cmd, args) => {
    const a = args as Record<string, unknown>;
    calls.push({ cmd, args: a });
    switch (cmd) {
      case "list_clients":
        return [client("c1", "Ромашка"), client("c2", "Василёк", { defaultCurrency: "EUR" })];
      case "create_project":
        if (createError) throw createError;
        return project("p1", (a.input as ProjectInput).name);
      case "update_project":
        return project(a.id as string, (a.input as ProjectInput).name);
    }
  });
});

afterEach(() => {
  clearMocks();
  setAppLocale("en");
});

const box = (name: string) => screen.getByRole("textbox", { name });
const input = (cmd: string) => calls.find((c) => c.cmd === cmd)!.args.input as ProjectInput;

async function choose(label: string, option: string) {
  await userEvent.click(screen.getByRole("combobox", { name: label }));
  await userEvent.click(screen.getByRole("option", { name: option }));
}

describe("ProjectForm", () => {
  it("pre-selects the client when created from a client card", async () => {
    const onsaved = vi.fn();
    render(ProjectForm, { clientId: "c2", onsaved });
    await waitFor(() =>
      expect(screen.getByRole("combobox", { name: "Client" })).toHaveTextContent("Василёк"),
    );
    await userEvent.type(box("Name"), "Лендинг");
    await userEvent.type(box("Budget"), "50000");
    await userEvent.click(screen.getByRole("button", { name: "Save" }));
    await waitFor(() => expect(onsaved).toHaveBeenCalledOnce());
    expect(input("create_project")).toEqual({
      clientId: "c2",
      name: "Лендинг",
      descriptionMd: "",
      status: "active",
      billingType: "fixed",
      currency: null,
      rateMinor: null,
      budgetMinor: 5_000_000,
      startDate: null,
      deadline: null,
    });
  });

  it("validates before saving, in the current language", async () => {
    render(ProjectForm, { onsaved: vi.fn() });
    await screen.findByRole("combobox", { name: "Client" });
    await userEvent.click(screen.getByRole("button", { name: "Save" }));
    expect(screen.getByRole("combobox", { name: "Client" })).toHaveAccessibleDescription(
      "This field is required",
    );
    expect(box("Name")).toHaveAccessibleDescription("This field is required");
    expect(box("Budget")).toHaveAccessibleDescription("This field is required");
    expect(calls.some((c) => c.cmd === "create_project")).toBe(false);

    setAppLocale("ru");
    await waitFor(() => expect(box("Название")).toHaveAccessibleDescription("Обязательное поле"));
  });

  it("switches between budget and hourly rate", async () => {
    render(ProjectForm, { clientId: "c1", onsaved: vi.fn() });
    await screen.findByRole("combobox", { name: "Client" });
    expect(box("Budget")).toBeInTheDocument();
    await userEvent.click(screen.getByRole("radio", { name: "Hourly" }));
    expect(screen.queryByRole("textbox", { name: "Budget" })).not.toBeInTheDocument();
    expect(box("Hourly rate")).toHaveAccessibleDescription("Leave empty to use the client's rate");

    await userEvent.type(box("Name"), "Поддержка");
    await userEvent.click(screen.getByRole("button", { name: "Save" }));
    await waitFor(() => expect(calls.some((c) => c.cmd === "create_project")).toBe(true));
    expect(input("create_project")).toMatchObject({
      billingType: "hourly",
      rateMinor: null,
      budgetMinor: null,
    });
  });

  it("shows backend validation errors under the field", async () => {
    createError = { code: "validation.before_start", params: { field: "deadline" } };
    const onsaved = vi.fn();
    render(ProjectForm, { clientId: "c1", onsaved });
    await screen.findByRole("combobox", { name: "Client" });
    await userEvent.type(box("Name"), "Сайт");
    await userEvent.type(box("Budget"), "100");
    await userEvent.click(screen.getByRole("button", { name: "Save" }));
    await waitFor(() => expect(screen.getByText("Cannot be before the start")).toBeInTheDocument());
    expect(onsaved).not.toHaveBeenCalled();
  });

  it("edits an existing project", async () => {
    const onsaved = vi.fn();
    render(ProjectForm, {
      project: project("p1", "Сайт", { descriptionMd: "# План", deadline: "2024-05-01" }),
      onsaved,
    });
    await waitFor(() => expect(box("Name")).toHaveValue("Сайт"));
    expect(editorText(screen.getByRole("textbox", { name: "Description" }))).toBe("# План");
    typeInEditor(screen.getByRole("textbox", { name: "Description" }), "\n- макет");
    await choose("Status", "Completed");
    await userEvent.click(screen.getByRole("button", { name: "Save" }));
    await waitFor(() => expect(onsaved).toHaveBeenCalledOnce());
    const sent = calls.find((c) => c.cmd === "update_project")!;
    expect(sent.args.id).toBe("p1");
    expect(sent.args.input).toMatchObject({
      status: "completed",
      descriptionMd: "# План\n- макет",
      deadline: "2024-05-01",
      currency: "RUB",
    });
  });

  it("has no a11y violations", async () => {
    const { container } = render(ProjectForm, { clientId: "c1", onsaved: vi.fn() });
    await screen.findByRole("combobox", { name: "Client" });
    await expectNoA11yViolations(container);
  });
});
