import { cleanup, render, screen, waitFor, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { clearMocks, mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Project_Serialize as Project } from "$lib/api/bindings";
import DialogHost from "$lib/ui/DialogHost.svelte";
import { resetDialogs } from "$lib/ui/dialogs.svelte";
import { expectNoA11yViolations } from "../../../test/a11y";
import { project } from "./fixtures";
import ProjectCard from "./ProjectCard.svelte";

const goto = vi.hoisted(() => vi.fn());
vi.mock("$app/navigation", () => ({ goto }));

let current: Project;
let calls: { cmd: string; args: Record<string, unknown> }[] = [];

beforeEach(() => {
  calls = [];
  current = project("p1", "Лендинг", {
    status: "active",
    descriptionMd: "## Цель\n\nЗапустить **до лета**",
    startDate: "2024-03-01",
    deadline: "2024-05-31",
  });
  mockWindows("main");
  mockIPC(
    (cmd, args) => {
      const a = (args ?? {}) as Record<string, unknown>;
      calls.push({ cmd, args: a });
      switch (cmd) {
        case "get_project":
          if (a.id !== current.id) throw { code: "project.not_found", params: { id: a.id } };
          return current;
        case "archive_project":
          current = { ...current, archivedAt: 1 };
          return current;
        case "restore_project":
          current = { ...current, archivedAt: null };
          return current;
        case "delete_project":
          return null;
        case "list_notes":
        case "list_custom_fields":
        case "list_tags":
        case "list_tasks":
        case "list_board_columns":
        case "list_secrets":
        case "list_attachments":
          return [];
        case "vault_status":
          return "unlocked";
      }
    },
    { shouldMockEvents: true },
  );
});

afterEach(() => {
  // Unmount first: components unsubscribe from mocked events on destroy.
  cleanup();
  clearMocks();
  resetDialogs();
  goto.mockReset();
});

const commands = () => calls.map((c) => c.cmd);

describe("ProjectCard", () => {
  it("shows status, client, dates, budget and the description", async () => {
    render(ProjectCard, { id: "p1" });
    expect(await screen.findByRole("heading", { level: 1, name: "Лендинг" })).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "Ромашка" })).toHaveAttribute("href", "/clients/c1");
    const overview = screen.getByRole("tabpanel");
    expect(within(overview).getByText("Active")).toBeInTheDocument();
    expect(within(overview).getByText("Fixed price")).toBeInTheDocument();
    expect(within(overview).getByText(/100,000\.00/)).toBeInTheDocument();
    expect(within(overview).getByText("Mar 1, 2024")).toBeInTheDocument();
    expect(within(overview).getByText("May 31, 2024")).toBeInTheDocument();
    expect(within(overview).getByText("до лета").tagName).toBe("STRONG");
  });

  it("shows the hourly rate of hourly projects", async () => {
    current = project("p1", "Поддержка", {
      billingType: "hourly",
      currency: "EUR",
      rateMinor: 5_000,
      budgetMinor: null,
    });
    render(ProjectCard, { id: "p1" });
    expect(await screen.findByText(/€50\.00\/h/)).toBeInTheDocument();
    expect(screen.getByText("No description")).toBeInTheDocument();
  });

  it("has notes, fields and files tabs; the rest are placeholders", async () => {
    render(ProjectCard, { id: "p1" });
    await userEvent.click(await screen.findByRole("tab", { name: "Notes" }));
    expect(await screen.findByRole("textbox", { name: "New note" })).toBeInTheDocument();
    expect(calls.find((c) => c.cmd === "list_notes")!.args.owner).toEqual({
      type: "project",
      id: "p1",
    });
    await userEvent.click(screen.getByRole("tab", { name: "Fields" }));
    expect(await screen.findByText("No custom fields yet")).toBeInTheDocument();
    await userEvent.click(screen.getByRole("tab", { name: "Files" }));
    expect(await screen.findByText("No files yet")).toBeInTheDocument();
    expect(calls.find((c) => c.cmd === "list_attachments")!.args.owner).toEqual({
      type: "project",
      id: "p1",
    });
    for (const name of ["Time", "Finance"]) {
      await userEvent.click(screen.getByRole("tab", { name }));
      expect(screen.getByRole("tabpanel")).toHaveTextContent("This section is coming soon.");
    }
  });

  it("lists the project's tasks and creates one for it", async () => {
    render(ProjectCard, { id: "p1" });
    await userEvent.click(await screen.findByRole("tab", { name: "Tasks" }));
    expect(await screen.findByText("No tasks yet")).toBeInTheDocument();
    expect(calls.find((c) => c.cmd === "list_tasks")!.args.filter).toMatchObject({
      scope: { type: "project", id: "p1" },
    });
    await userEvent.click(screen.getByRole("button", { name: "New task" }));
    expect(goto).toHaveBeenCalledWith("/tasks/new?project=p1");
  });

  it("lists the project's secrets behind the vault", async () => {
    render(ProjectCard, { id: "p1" });
    await userEvent.click(await screen.findByRole("tab", { name: "Secrets" }));
    expect(await screen.findByText("No secrets yet")).toBeInTheDocument();
    expect(calls.find((c) => c.cmd === "list_secrets")!.args.filter).toMatchObject({
      projectId: "p1",
    });
    await userEvent.click(screen.getByRole("button", { name: "New secret" }));
    expect(goto).toHaveBeenCalledWith("/vault/new?project=p1");
  });

  it("opens the edit page", async () => {
    render(ProjectCard, { id: "p1" });
    await userEvent.click(await screen.findByRole("button", { name: "Edit" }));
    expect(goto).toHaveBeenCalledWith("/projects/p1/edit");
  });

  it("archives, restores and deletes after confirmation", async () => {
    render(DialogHost);
    render(ProjectCard, { id: "p1" });
    const menu = async (item: string) => {
      await userEvent.click(await screen.findByRole("button", { name: "More actions" }));
      await userEvent.click(screen.getByRole("menuitem", { name: item }));
    };
    await menu("Archive");
    expect(await screen.findByText("Archived")).toBeInTheDocument();
    await menu("Restore from archive");
    await waitFor(() => expect(screen.queryByText("Archived")).not.toBeInTheDocument());

    await menu("Delete project");
    const dialog = await screen.findByRole("alertdialog", { name: "Delete project Лендинг?" });
    await userEvent.click(within(dialog).getByRole("button", { name: "Delete" }));
    await waitFor(() => expect(goto).toHaveBeenCalledWith("/projects"));
    expect(commands()).toContain("delete_project");
  });

  it("shows an error for an unknown project", async () => {
    render(ProjectCard, { id: "missing" });
    expect(await screen.findByRole("alert")).toHaveTextContent("Project not found");
  });

  it("has no a11y violations", async () => {
    const { container } = render(ProjectCard, { id: "p1" });
    await screen.findByRole("heading", { level: 1 });
    await expectNoA11yViolations(container);
  });
});
