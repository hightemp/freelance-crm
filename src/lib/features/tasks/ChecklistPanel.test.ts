import { render, screen, waitFor, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { ChecklistItem } from "$lib/api/bindings";
import { expectNoA11yViolations } from "../../../test/a11y";
import ChecklistPanel from "./ChecklistPanel.svelte";

let items: ChecklistItem[] = [];
let next = 0;

beforeEach(() => {
  next = 0;
  items = [
    { id: "i1", text: "Макет", done: true },
    { id: "i2", text: "Вёрстка", done: false },
    { id: "i3", text: "Тесты", done: false },
  ];
  mockIPC((cmd, args) => {
    const a = args as Record<string, unknown>;
    switch (cmd) {
      case "list_checklist":
        expect(a.taskId).toBe("t1");
        return items;
      case "add_checklist_item": {
        const item = { id: `n${++next}`, text: a.text as string, done: false };
        items = [...items, item];
        return item;
      }
      case "update_checklist_item":
        items = items.map((i) =>
          i.id === a.id ? { ...i, text: a.text as string, done: a.done as boolean } : i,
        );
        return items.find((i) => i.id === a.id);
      case "delete_checklist_item":
        items = items.filter((i) => i.id !== a.id);
        return null;
      case "reorder_checklist":
        items = (a.ids as string[]).map((id) => items.find((i) => i.id === id)!);
        return items;
    }
  });
});

afterEach(() => clearMocks());

const rows = () => screen.getAllByRole("listitem");
// Buttons in a row are icon-only, so the row text is the item text.
const texts = () => rows().map((r) => r.textContent?.trim());

describe("ChecklistPanel", () => {
  it("shows items and the progress", async () => {
    render(ChecklistPanel, { taskId: "t1" });
    await waitFor(() => expect(rows()).toHaveLength(3));
    expect(screen.getByRole("checkbox", { name: "Макет" })).toBeChecked();
    expect(screen.getByRole("progressbar", { name: "Checklist" })).toHaveAttribute(
      "aria-valuenow",
      "33",
    );
    expect(screen.getByText("33% done")).toBeInTheDocument();
  });

  it("adds an item with Enter and reports changes", async () => {
    const onchange = vi.fn();
    render(ChecklistPanel, { taskId: "t1", onchange });
    await waitFor(() => expect(rows()).toHaveLength(3));
    await userEvent.type(screen.getByRole("textbox", { name: "New item" }), "Деплой{Enter}");
    await waitFor(() => expect(rows()).toHaveLength(4));
    expect(screen.getByRole("checkbox", { name: "Деплой" })).not.toBeChecked();
    expect(screen.getByRole("textbox", { name: "New item" })).toHaveValue("");
    expect(onchange).toHaveBeenCalled();
  });

  it("checks items off", async () => {
    render(ChecklistPanel, { taskId: "t1" });
    await userEvent.click(await screen.findByRole("checkbox", { name: "Вёрстка" }));
    await waitFor(() => expect(screen.getByText("66% done")).toBeInTheDocument());
    expect(items[1].done).toBe(true);
  });

  it("deletes and reorders items", async () => {
    render(ChecklistPanel, { taskId: "t1" });
    await waitFor(() => expect(rows()).toHaveLength(3));
    await userEvent.click(screen.getByRole("button", { name: "Delete item Макет" }));
    await waitFor(() => expect(rows()).toHaveLength(2));
    await userEvent.click(within(rows()[1]).getByRole("button", { name: "Move up" }));
    await waitFor(() => expect(texts()).toEqual(["Тесты", "Вёрстка"]));
  });

  it("shows an empty state without progress", async () => {
    items = [];
    render(ChecklistPanel, { taskId: "t1" });
    expect(await screen.findByText("No items yet")).toBeInTheDocument();
    expect(screen.queryByRole("progressbar")).not.toBeInTheDocument();
  });

  it("has no a11y violations", async () => {
    const { container } = render(ChecklistPanel, { taskId: "t1" });
    await waitFor(() => expect(rows()).toHaveLength(3));
    await expectNoA11yViolations(container);
  });
});
