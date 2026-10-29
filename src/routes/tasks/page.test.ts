import { render, screen } from "@testing-library/svelte";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, describe, expect, it } from "vitest";
import Page from "./+page.svelte";

afterEach(() => clearMocks());

describe("tasks page", () => {
  it("shows the task list", async () => {
    mockIPC(() => []);
    render(Page);
    expect(screen.getByRole("heading", { level: 1 })).toHaveTextContent("Tasks");
    expect(await screen.findByText("No tasks yet")).toBeInTheDocument();
  });
});
