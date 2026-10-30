import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { SearchHit } from "$lib/api/bindings";
import { timer, timerPanel } from "../time/timer.svelte";
import { expectNoA11yViolations } from "../../../test/a11y";
import CommandPalette from "./CommandPalette.svelte";

const goto = vi.hoisted(() => vi.fn());
vi.mock("$app/navigation", () => ({ goto }));

let calls: { cmd: string; args: Record<string, unknown> }[] = [];

const HITS: SearchHit[] = [
  { kind: "client", id: "c1", owner: null, title: "ООО Ромашка", snippet: null },
  {
    kind: "note",
    id: "n1",
    owner: { type: "client", id: "c1" },
    title: "Скидка",
    snippet: "Скидка для [Ромашки]",
  },
];

beforeEach(() => {
  calls = [];
  timer.running = null;
  timer.loaded = true;
  timerPanel.open = false;
  mockIPC((cmd, args) => {
    calls.push({ cmd, args: (args ?? {}) as Record<string, unknown> });
    if (cmd === "search") return HITS;
    if (cmd === "vault_lock") return null;
  });
});

afterEach(() => {
  cleanup();
  clearMocks();
  goto.mockReset();
});

function open() {
  render(CommandPalette);
  fireEvent.keyDown(window, { key: "k", ctrlKey: true });
  return screen.getByRole("combobox", { name: "Search or run a command" });
}

const active = () => {
  const input = screen.getByRole("combobox");
  const id = input.getAttribute("aria-activedescendant");
  return id ? document.getElementById(id)?.textContent : null;
};

describe("CommandPalette", () => {
  it("opens with Ctrl/Cmd+K and closes with Escape", async () => {
    const input = open();
    await waitFor(() => expect(input).toHaveFocus());
    await userEvent.keyboard("{Escape}");
    expect(screen.queryByRole("combobox")).toBeNull();
    fireEvent.keyDown(window, { key: "k", metaKey: true });
    expect(screen.getByRole("combobox")).toBeInTheDocument();
  });

  it("finds records and opens the chosen one with the keyboard", async () => {
    const input = open();
    await userEvent.type(input, "ромашк");
    expect(await screen.findByRole("option", { name: /ООО Ромашка/ })).toBeInTheDocument();
    expect(calls.find((c) => c.cmd === "search")?.args).toEqual({ query: "ромашк", limit: 20 });
    expect(screen.getByRole("option", { name: /Скидка/ })).toHaveTextContent("Note");
    expect(active()).toMatch(/ООО Ромашка/);
    await userEvent.keyboard("{ArrowDown}");
    expect(active()).toMatch(/Скидка/);
    await userEvent.keyboard("{ArrowDown}");
    expect(active()).toMatch(/ООО Ромашка/);
    await userEvent.keyboard("{ArrowUp}");
    await userEvent.keyboard("{Enter}");
    expect(goto).toHaveBeenCalledWith("/clients/c1?tab=notes");
    expect(screen.queryByRole("combobox")).toBeNull();
  });

  it("runs commands: a new task", async () => {
    const input = open();
    await userEvent.type(input, "new task");
    await waitFor(() => expect(active()).toBe("New task"));
    await userEvent.keyboard("{Enter}");
    expect(goto).toHaveBeenCalledWith("/tasks/new");
  });

  it("runs commands: lock the vault", async () => {
    const input = open();
    await userEvent.type(input, "lock");
    await userEvent.click(await screen.findByRole("option", { name: "Lock the vault" }));
    await waitFor(() => expect(calls.some((c) => c.cmd === "vault_lock")).toBe(true));
  });

  it("starts the timer from its panel", async () => {
    const input = open();
    await userEvent.type(input, "timer");
    await userEvent.click(await screen.findByRole("option", { name: "Start timer" }));
    expect(timerPanel.open).toBe(true);
  });

  it("lists commands before typing", () => {
    open();
    expect(screen.getAllByRole("option").map((o) => o.textContent)).toContain("New client");
  });

  it("has no a11y violations", async () => {
    const input = open();
    await userEvent.type(input, "ром");
    await screen.findByRole("option", { name: /ООО Ромашка/ });
    await expectNoA11yViolations(document.body);
  });
});
