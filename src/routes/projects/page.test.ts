import { render, screen, waitFor } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { client } from "$lib/features/projects/fixtures";
import NewPage from "./new/+page.svelte";
import Page from "./+page.svelte";

const goto = vi.hoisted(() => vi.fn());
const pageState = vi.hoisted(() => ({ url: new URL("http://localhost/projects/new") }));
vi.mock("$app/navigation", () => ({ goto }));
vi.mock("$app/state", () => ({ page: pageState }));

beforeEach(() =>
  mockIPC((cmd) => {
    if (cmd === "list_clients") return [client("c1", "Ромашка"), client("c2", "Василёк")];
    return [];
  }),
);
afterEach(() => {
  clearMocks();
  goto.mockReset();
});

describe("projects page", () => {
  it("lists projects and opens the new project form", async () => {
    render(Page);
    expect(screen.getByRole("heading", { level: 1 })).toHaveTextContent("Projects");
    expect(await screen.findByText("No projects yet")).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "New project" }));
    expect(goto).toHaveBeenCalledWith("/projects/new");
  });
});

describe("new project page", () => {
  it("takes the client from the query", async () => {
    pageState.url = new URL("http://localhost/projects/new?client=c2");
    render(NewPage);
    await waitFor(() =>
      expect(screen.getByRole("combobox", { name: "Client" })).toHaveTextContent("Василёк"),
    );
  });
});
