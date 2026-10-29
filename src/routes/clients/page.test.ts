import { render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import NewPage from "./new/+page.svelte";
import Page from "./+page.svelte";

const goto = vi.hoisted(() => vi.fn());
vi.mock("$app/navigation", () => ({ goto }));

beforeEach(() => mockIPC(() => []));
afterEach(() => {
  clearMocks();
  goto.mockReset();
});

describe("clients page", () => {
  it("shows the heading and the client list", async () => {
    render(Page);
    expect(screen.getByRole("heading", { level: 1 })).toHaveTextContent("Clients");
    expect(await screen.findByText("No clients yet")).toBeInTheDocument();
  });

  it("opens the new client form", async () => {
    render(Page);
    await userEvent.click(screen.getByRole("button", { name: "New client" }));
    expect(goto).toHaveBeenCalledWith("/clients/new");
  });
});

describe("new client page", () => {
  it("returns to the list on cancel", async () => {
    render(NewPage);
    expect(screen.getByRole("heading", { level: 1 })).toHaveTextContent("New client");
    await userEvent.click(screen.getByRole("button", { name: "Cancel" }));
    expect(goto).toHaveBeenCalledWith("/clients");
  });
});
