import { render, screen } from "@testing-library/svelte";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, describe, expect, it } from "vitest";
import Page from "./+page.svelte";

describe("home page", () => {
  afterEach(() => clearMocks());

  it("renders the home heading", () => {
    mockIPC(() => null);
    render(Page);
    expect(screen.getByRole("heading", { level: 1 })).toHaveTextContent("Home");
  });
});
