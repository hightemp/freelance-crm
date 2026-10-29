import { render, screen, waitFor } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { client, project } from "$lib/features/projects/fixtures";
import { COLUMNS } from "$lib/features/tasks/fixtures";
import NewPage from "./new/+page.svelte";
import Page from "./+page.svelte";

const goto = vi.hoisted(() => vi.fn());
const pageState = vi.hoisted(() => ({ url: new URL("http://localhost/tasks/new") }));
vi.mock("$app/navigation", () => ({ goto }));
vi.mock("$app/state", () => ({ page: pageState }));

beforeEach(() =>
  mockIPC((cmd) => {
    if (cmd === "list_board_columns") return COLUMNS;
    if (cmd === "list_clients") return [client("c1", "Ромашка")];
    if (cmd === "list_projects") return [project("p1", "Сайт")];
    return [];
  }),
);
afterEach(() => {
  clearMocks();
  goto.mockReset();
});

describe("tasks page", () => {
  it("switches between the list and the board", async () => {
    render(Page);
    expect(screen.getByRole("heading", { level: 1 })).toHaveTextContent("Tasks");
    expect(await screen.findByText("No tasks yet")).toBeInTheDocument();
    await userEvent.click(screen.getByRole("tab", { name: "Board" }));
    expect(await screen.findByRole("region", { name: "To do" })).toBeInTheDocument();
  });

  it("opens the new task form", async () => {
    render(Page);
    await userEvent.click(screen.getByRole("button", { name: "New task" }));
    expect(goto).toHaveBeenCalledWith("/tasks/new");
  });
});

describe("new task page", () => {
  it("takes the project from the query", async () => {
    pageState.url = new URL("http://localhost/tasks/new?project=p1");
    render(NewPage);
    await waitFor(() =>
      expect(screen.getByRole("combobox", { name: "Project" })).toHaveTextContent("Сайт"),
    );
    expect(screen.getByRole("combobox", { name: "Client" })).toHaveTextContent("Ромашка");
  });
});
