import { render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, describe, expect, it } from "vitest";
import { COLUMNS } from "$lib/features/tasks/fixtures";
import Page from "./+page.svelte";

afterEach(() => clearMocks());

describe("tasks page", () => {
  it("switches between the list and the board", async () => {
    mockIPC((cmd) => (cmd === "list_board_columns" ? COLUMNS : []));
    render(Page);
    expect(screen.getByRole("heading", { level: 1 })).toHaveTextContent("Tasks");
    expect(await screen.findByText("No tasks yet")).toBeInTheDocument();
    await userEvent.click(screen.getByRole("tab", { name: "Board" }));
    expect(await screen.findByRole("region", { name: "To do" })).toBeInTheDocument();
  });
});
