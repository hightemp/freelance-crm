import { render, screen, waitFor, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
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
  mockIPC((cmd, args) => {
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
        return [];
    }
  });
});

afterEach(() => {
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

  it("has notes and fields tabs; the rest are placeholders", async () => {
    render(ProjectCard, { id: "p1" });
    await userEvent.click(await screen.findByRole("tab", { name: "Notes" }));
    expect(await screen.findByRole("textbox", { name: "New note" })).toBeInTheDocument();
    expect(calls.find((c) => c.cmd === "list_notes")!.args.owner).toEqual({
      type: "project",
      id: "p1",
    });
    await userEvent.click(screen.getByRole("tab", { name: "Fields" }));
    expect(await screen.findByText("No custom fields yet")).toBeInTheDocument();
    for (const name of ["Tasks", "Time", "Secrets", "Files", "Finance"]) {
      await userEvent.click(screen.getByRole("tab", { name }));
      expect(screen.getByRole("tabpanel")).toHaveTextContent("This section is coming soon.");
    }
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
