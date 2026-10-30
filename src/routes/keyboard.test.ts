// The main scenario with the keyboard only (NFR-7): find the command in
// the palette, then fill and save the new client form with Tab and Enter.
// (In e2e the embedded WebDriver sends untrusted key events, which the
// webview does not act on, so this runs here with user-event.)
import { cleanup, render, screen, waitFor } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import CommandPalette from "$lib/features/search/CommandPalette.svelte";
import NewClient from "./clients/new/+page.svelte";

const goto = vi.hoisted(() => vi.fn());
vi.mock("$app/navigation", () => ({ goto }));
const pageState = vi.hoisted(() => ({ url: new URL("http://localhost/clients/new"), params: {} }));
vi.mock("$app/state", () => ({ page: pageState }));

let calls: { cmd: string; args: Record<string, unknown> }[] = [];

beforeEach(() => {
  calls = [];
  mockIPC((cmd, args) => {
    calls.push({ cmd, args: (args ?? {}) as Record<string, unknown> });
    switch (cmd) {
      case "create_client":
        return { id: "c1", displayName: "Ромашка" };
      case "search":
      case "replace_contact_methods":
      case "list_country_schemas":
        return [];
    }
  });
});

afterEach(() => {
  cleanup();
  clearMocks();
  goto.mockReset();
});

/** Presses Tab until the focused element matches, at most `max` times. */
async function tabTo(user: ReturnType<typeof userEvent.setup>, match: (el: Element) => boolean) {
  for (let i = 0; i < 20; i++) {
    await user.tab();
    if (document.activeElement && match(document.activeElement)) return document.activeElement;
  }
  throw new Error("not reachable with Tab");
}

describe("keyboard only", () => {
  it("finds the command and creates a client", async () => {
    const user = userEvent.setup();
    render(CommandPalette);
    await user.keyboard("{Control>}k{/Control}");
    const input = await screen.findByRole("combobox", { name: "Search or run a command" });
    await waitFor(() => expect(input).toHaveFocus());
    await user.keyboard("new client");
    await user.keyboard("{Enter}");
    expect(goto).toHaveBeenCalledWith("/clients/new");
    cleanup();

    render(NewClient);
    const name = await tabTo(user, (el) => el.matches("input[type=text]"));
    expect(name).toHaveAccessibleName(/^Short name/);
    await user.keyboard("Ромашка");
    await tabTo(user, (el) => el.textContent?.trim() === "Save");
    await user.keyboard("{Enter}");
    await waitFor(() =>
      expect(calls.find((c) => c.cmd === "create_client")?.args.input).toMatchObject({
        kind: "company",
        companyName: "Ромашка",
      }),
    );
  });
});
