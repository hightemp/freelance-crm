import { render, screen, waitFor, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import type { EntityRef, Note } from "$lib/api/bindings";
import DialogHost from "$lib/ui/DialogHost.svelte";
import { resetDialogs } from "$lib/ui/dialogs.svelte";
import { expectNoA11yViolations } from "../../../test/a11y";
import { editorText, replaceInEditor, typeInEditor } from "../../../test/editor";
import NotesPanel from "./NotesPanel.svelte";

const OWNER: EntityRef = { type: "client", id: "c1" };
let notes: Note[] = [];
let clock = 1_000;

/** Minimal in-memory backend with the same ordering rules as crm_core. */
function sorted() {
  return [...notes].sort(
    (a, b) => Number(b.pinned) - Number(a.pinned) || b.updatedAt - a.updatedAt,
  );
}

beforeEach(() => {
  clock = 1_000;
  notes = [
    { id: "n1", bodyMd: "Старая заметка", pinned: false, createdAt: 1, updatedAt: 1 },
    { id: "n2", bodyMd: "Важное", pinned: true, createdAt: 2, updatedAt: 2 },
  ];
  mockIPC((cmd, args) => {
    const a = args as Record<string, unknown>;
    const input = a.input as { bodyMd: string; pinned: boolean } | undefined;
    switch (cmd) {
      case "list_notes":
        expect(a.owner).toEqual(OWNER);
        return sorted();
      case "create_note": {
        const note = { id: `n${++clock}`, ...input!, createdAt: clock, updatedAt: clock };
        notes.push(note);
        return note;
      }
      case "update_note": {
        const note = notes.find((n) => n.id === a.id)!;
        Object.assign(note, input, { updatedAt: ++clock });
        return note;
      }
      case "set_note_pinned": {
        const note = notes.find((n) => n.id === a.id)!;
        note.pinned = a.pinned as boolean;
        return note;
      }
      case "delete_note":
        notes = notes.filter((n) => n.id !== a.id);
        return null;
    }
  });
});

afterEach(() => {
  clearMocks();
  resetDialogs();
});

const items = () => screen.getAllByRole("article");
const bodies = () => items().map((el) => within(el).getByTestId("note-body").textContent?.trim());

describe("NotesPanel", () => {
  it("lists pinned notes first", async () => {
    render(NotesPanel, { owner: OWNER });
    await waitFor(() => expect(bodies()).toEqual(["Важное", "Старая заметка"]));
    expect(within(items()[0]).getByText("Pinned")).toBeInTheDocument();
  });

  it("adds a note", async () => {
    render(NotesPanel, { owner: OWNER });
    typeInEditor(screen.getByRole("textbox", { name: "New note" }), "Созвон в среду");
    await userEvent.click(screen.getByRole("button", { name: "Add note" }));
    await waitFor(() => expect(bodies()).toEqual(["Важное", "Созвон в среду", "Старая заметка"]));
    expect(editorText(screen.getByRole("textbox", { name: "New note" }))).toBe("");
  });

  it("does not add a blank note", async () => {
    render(NotesPanel, { owner: OWNER });
    await waitFor(() => expect(items()).toHaveLength(2));
    expect(screen.getByRole("button", { name: "Add note" })).toBeDisabled();
  });

  it("pins and unpins", async () => {
    render(NotesPanel, { owner: OWNER });
    await waitFor(() => expect(items()).toHaveLength(2));
    await userEvent.click(within(items()[1]).getByRole("button", { name: "Pin" }));
    await waitFor(() => expect(within(items()[1]).getByText("Pinned")).toBeInTheDocument());
    await userEvent.click(within(items()[0]).getByRole("button", { name: "Unpin" }));
    await waitFor(() => expect(bodies()).toEqual(["Старая заметка", "Важное"]));
  });

  it("edits a note", async () => {
    render(NotesPanel, { owner: OWNER });
    await waitFor(() => expect(items()).toHaveLength(2));
    await userEvent.click(within(items()[1]).getByRole("button", { name: "Edit" }));
    replaceInEditor(screen.getByRole("textbox", { name: "Edit note" }), "Обновлённая");
    await userEvent.click(screen.getByRole("button", { name: "Save" }));
    await waitFor(() => expect(bodies()).toContain("Обновлённая"));
  });

  it("deletes a note after confirmation", async () => {
    render(DialogHost);
    render(NotesPanel, { owner: OWNER });
    await waitFor(() => expect(items()).toHaveLength(2));
    await userEvent.click(within(items()[1]).getByRole("button", { name: "Delete" }));
    const dialog = await screen.findByRole("alertdialog", { name: "Delete this note?" });
    await userEvent.click(within(dialog).getByRole("button", { name: "Delete" }));
    await waitFor(() => expect(bodies()).toEqual(["Важное"]));
  });

  it("renders note bodies as Markdown", async () => {
    notes = [{ id: "n1", bodyMd: "**Важно**", pinned: false, createdAt: 1, updatedAt: 1 }];
    render(NotesPanel, { owner: OWNER });
    expect((await screen.findByText("Важно")).tagName).toBe("STRONG");
  });

  it("shows an empty state", async () => {
    notes = [];
    render(NotesPanel, { owner: OWNER });
    expect(await screen.findByText("No notes yet")).toBeInTheDocument();
  });

  it("has no a11y violations", async () => {
    const { container } = render(NotesPanel, { owner: OWNER });
    await waitFor(() => expect(items()).toHaveLength(2));
    await expectNoA11yViolations(container);
  });
});
