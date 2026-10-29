import { render, screen } from "@testing-library/svelte";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, describe, expect, it } from "vitest";
import Page from "./+page.svelte";

afterEach(() => clearMocks());

describe("clients page", () => {
  it("shows the heading and the client list", async () => {
    mockIPC(() => []);
    render(Page);
    expect(screen.getByRole("heading", { level: 1 })).toHaveTextContent("Clients");
    expect(await screen.findByText("No clients yet")).toBeInTheDocument();
  });
});
