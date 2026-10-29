import { render, screen } from "@testing-library/svelte";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, describe, expect, it } from "vitest";
import { resetSchemas } from "$lib/features/requisites/schemas";
import Page from "./+page.svelte";

afterEach(() => {
  clearMocks();
  resetSchemas();
});

describe("settings page", () => {
  it("shows app settings and the profile requisites", async () => {
    const requests: unknown[] = [];
    mockIPC((cmd, args) => {
      if (cmd === "list_country_schemas") return [];
      if (cmd === "get_requisites") {
        requests.push((args as { owner: unknown }).owner);
        return null;
      }
    });
    render(Page);
    expect(screen.getByRole("heading", { level: 1 })).toHaveTextContent("Settings");
    expect(screen.getByRole("heading", { level: 2, name: "Profile" })).toBeInTheDocument();
    expect(await screen.findByRole("textbox", { name: "Name on documents" })).toBeInTheDocument();
    expect(requests).toEqual([{ type: "profile" }]);
  });
});
