import { render, screen, waitFor, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import type { CustomField, CustomFieldInput, EntityRef } from "$lib/api/bindings";
import DialogHost from "$lib/ui/DialogHost.svelte";
import { resetDialogs } from "$lib/ui/dialogs.svelte";
import { expectNoA11yViolations } from "../../../test/a11y";
import FieldsPanel from "./FieldsPanel.svelte";

const OWNER: EntityRef = { type: "client", id: "c1" };
let fields: CustomField[] = [];
let reorders: string[][] = [];
let next = 0;

beforeEach(() => {
  next = 0;
  reorders = [];
  fields = [
    { id: "f1", key: "ИНН", value: "7707083893", fieldType: "text" },
    { id: "f2", key: "Сайт", value: "https://example.com/", fieldType: "url" },
  ];
  mockIPC((cmd, args) => {
    const a = args as Record<string, unknown>;
    const input = a.input as CustomFieldInput | undefined;
    switch (cmd) {
      case "list_custom_fields":
        expect(a.owner).toEqual(OWNER);
        return fields;
      case "create_custom_field": {
        if (input!.fieldType === "number" && Number.isNaN(Number(input!.value))) {
          throw { code: "validation.number", params: { field: "value" } };
        }
        const field = { id: `new${++next}`, ...input! } as CustomField;
        fields = [...fields, field];
        return field;
      }
      case "update_custom_field": {
        fields = fields.map((f) => (f.id === a.id ? ({ ...f, ...input } as CustomField) : f));
        return fields.find((f) => f.id === a.id);
      }
      case "reorder_custom_fields": {
        const ids = a.ids as string[];
        reorders.push(ids);
        fields = ids.map((id) => fields.find((f) => f.id === id)!);
        return fields;
      }
      case "delete_custom_field":
        fields = fields.filter((f) => f.id !== a.id);
        return null;
    }
  });
});

afterEach(() => {
  clearMocks();
  resetDialogs();
});

const rows = () => screen.getAllByRole("listitem");
const keys = () => rows().map((r) => within(r).getByTestId("field-key").textContent?.trim());

async function openAdd() {
  await userEvent.click(screen.getByRole("button", { name: "Add field" }));
  return screen.findByRole("dialog", { name: "New field" });
}

describe("FieldsPanel", () => {
  it("lists fields in order with their values", async () => {
    render(FieldsPanel, { owner: OWNER });
    await waitFor(() => expect(keys()).toEqual(["ИНН", "Сайт"]));
    expect(within(rows()[1]).getByText("https://example.com/")).toBeInTheDocument();
  });

  it("adds a typed field", async () => {
    render(FieldsPanel, { owner: OWNER });
    const dialog = await openAdd();
    await userEvent.type(within(dialog).getByRole("textbox", { name: "Name" }), "Сотрудников");
    await userEvent.click(within(dialog).getByRole("combobox", { name: "Type" }));
    await userEvent.click(screen.getByRole("option", { name: "Number" }));
    await userEvent.type(within(dialog).getByRole("textbox", { name: "Value" }), "12");
    await userEvent.click(within(dialog).getByRole("button", { name: "Save" }));
    await waitFor(() => expect(keys()).toEqual(["ИНН", "Сайт", "Сотрудников"]));
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  it("requires a name and shows backend errors under the value", async () => {
    render(FieldsPanel, { owner: OWNER });
    const dialog = await openAdd();
    await userEvent.click(within(dialog).getByRole("button", { name: "Save" }));
    expect(within(dialog).getByRole("textbox", { name: "Name" })).toHaveAccessibleDescription(
      "This field is required",
    );

    await userEvent.type(within(dialog).getByRole("textbox", { name: "Name" }), "Число");
    await userEvent.click(within(dialog).getByRole("combobox", { name: "Type" }));
    await userEvent.click(screen.getByRole("option", { name: "Number" }));
    await userEvent.type(within(dialog).getByRole("textbox", { name: "Value" }), "abc");
    await userEvent.click(within(dialog).getByRole("button", { name: "Save" }));
    await waitFor(() =>
      expect(within(dialog).getByRole("textbox", { name: "Value" })).toHaveAccessibleDescription(
        "Invalid number",
      ),
    );
  });

  it("edits a field", async () => {
    render(FieldsPanel, { owner: OWNER });
    await waitFor(() => expect(rows()).toHaveLength(2));
    await userEvent.click(within(rows()[0]).getByRole("button", { name: "Edit" }));
    const dialog = await screen.findByRole("dialog", { name: "Edit field" });
    const value = within(dialog).getByRole("textbox", { name: "Value" });
    expect(value).toHaveValue("7707083893");
    await userEvent.clear(value);
    await userEvent.type(value, "500100732259");
    await userEvent.click(within(dialog).getByRole("button", { name: "Save" }));
    await waitFor(() => expect(screen.getByText("500100732259")).toBeInTheDocument());
  });

  it("reorders with move buttons", async () => {
    render(FieldsPanel, { owner: OWNER });
    await waitFor(() => expect(rows()).toHaveLength(2));
    expect(within(rows()[0]).getByRole("button", { name: "Move up" })).toBeDisabled();
    await userEvent.click(within(rows()[0]).getByRole("button", { name: "Move down" }));
    await waitFor(() => expect(keys()).toEqual(["Сайт", "ИНН"]));
    expect(reorders).toEqual([["f2", "f1"]]);
  });

  it("deletes after confirmation", async () => {
    render(DialogHost);
    render(FieldsPanel, { owner: OWNER });
    await waitFor(() => expect(rows()).toHaveLength(2));
    await userEvent.click(within(rows()[0]).getByRole("button", { name: "Delete" }));
    const dialog = await screen.findByRole("alertdialog", { name: "Delete field ИНН?" });
    await userEvent.click(within(dialog).getByRole("button", { name: "Delete" }));
    await waitFor(() => expect(keys()).toEqual(["Сайт"]));
  });

  it("shows an empty state", async () => {
    fields = [];
    render(FieldsPanel, { owner: OWNER });
    expect(await screen.findByText("No custom fields yet")).toBeInTheDocument();
  });

  it("has no a11y violations", async () => {
    const { container } = render(FieldsPanel, { owner: OWNER });
    await waitFor(() => expect(rows()).toHaveLength(2));
    await expectNoA11yViolations(container);
  });
});
