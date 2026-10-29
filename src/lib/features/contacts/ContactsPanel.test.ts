import { render, screen, waitFor, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import type {
  ContactInput_Deserialize as ContactInput,
  ContactMethodInput_Deserialize as ContactMethodInput,
  ContactMethod_Serialize as ContactMethod,
  Contact_Serialize as Contact,
  EntityRef,
} from "$lib/api/bindings";
import DialogHost from "$lib/ui/DialogHost.svelte";
import { resetDialogs } from "$lib/ui/dialogs.svelte";
import { expectNoA11yViolations } from "../../../test/a11y";
import ContactsPanel from "./ContactsPanel.svelte";

let contacts: Contact[] = [];
let methods: Record<string, ContactMethod[]> = {};
let calls: { cmd: string; args: Record<string, unknown> }[] = [];
let next = 0;

function contact(id: string, last: string, first: string, patch: Partial<Contact> = {}): Contact {
  return {
    id,
    clientId: "c1",
    lastName: last,
    firstName: first,
    middleName: null,
    position: null,
    note: null,
    isPrimary: false,
    displayName: `${last} ${first}`,
    createdAt: 0,
    updatedAt: 0,
    ...patch,
  };
}

function setPrimary(id: string) {
  contacts = contacts.map((c) => ({ ...c, isPrimary: c.id === id }));
}

beforeEach(() => {
  next = 0;
  calls = [];
  contacts = [
    contact("p1", "Сидоров", "Пётр", { isPrimary: true, position: "CTO" }),
    contact("p2", "Борисова", "Анна"),
  ];
  methods = { p1: [{ id: "m1", kind: "email", value: "petr@example.com", label: null }] };
  mockIPC((cmd, args) => {
    const a = args as Record<string, unknown>;
    calls.push({ cmd, args: a });
    const input = a.input as ContactInput | undefined;
    switch (cmd) {
      case "list_contacts":
        expect(a.clientId).toBe("c1");
        return [...contacts].sort((x, y) => Number(y.isPrimary) - Number(x.isPrimary));
      case "list_contact_methods":
        return methods[(a.owner as EntityRef).id] ?? [];
      case "create_contact": {
        const id = `new${++next}`;
        const created = contact(id, input!.lastName ?? "", input!.firstName ?? "", input!);
        contacts.push(created);
        if (input!.isPrimary) setPrimary(id);
        return created;
      }
      case "update_contact": {
        contacts = contacts.map((c) => (c.id === a.id ? { ...c, ...input } : c));
        if (input!.isPrimary) setPrimary(a.id as string);
        return contacts.find((c) => c.id === a.id);
      }
      case "set_primary_contact":
        setPrimary(a.id as string);
        return contacts.find((c) => c.id === a.id);
      case "replace_contact_methods": {
        const owner = a.owner as EntityRef;
        methods[owner.id] = (a.methods as ContactMethodInput[]).map((m, i) => ({
          ...m,
          label: m.label ?? null,
          id: `m-${owner.id}-${i}`,
        }));
        return methods[owner.id];
      }
      case "delete_contact":
        contacts = contacts.filter((c) => c.id !== a.id);
        return null;
    }
  });
});

afterEach(() => {
  clearMocks();
  resetDialogs();
});

const cards = () => screen.getAllByRole("article");
const names = () => cards().map((c) => within(c).getByRole("heading").textContent?.trim());
const card = (name: string) => cards().find((c) => c.textContent?.includes(name))!;

async function openAdd() {
  await userEvent.click(screen.getByRole("button", { name: "Add contact" }));
  return screen.findByRole("dialog", { name: "New contact" });
}

describe("ContactsPanel", () => {
  it("lists contacts with the primary one first", async () => {
    render(ContactsPanel, { clientId: "c1" });
    await waitFor(() => expect(names()).toEqual(["Сидоров Пётр", "Борисова Анна"]));
    const petr = card("Сидоров");
    expect(within(petr).getByText("Primary")).toBeInTheDocument();
    expect(within(petr).getByText("CTO")).toBeInTheDocument();
    expect(await within(petr).findByText("petr@example.com")).toBeInTheDocument();
  });

  it("adds a contact with contact methods", async () => {
    render(ContactsPanel, { clientId: "c1" });
    const dialog = await openAdd();
    const box = (name: string) => within(dialog).getByRole("textbox", { name });
    await userEvent.type(box("Last name"), "Иванова");
    await userEvent.type(box("First name"), "Мария");
    await userEvent.type(box("Position or role"), "Бухгалтер");
    await userEvent.click(within(dialog).getByRole("checkbox", { name: "Primary contact" }));
    await userEvent.click(within(dialog).getByRole("button", { name: "Add contact method" }));
    await userEvent.type(box("Value"), "maria@example.com");
    await userEvent.click(within(dialog).getByRole("button", { name: "Save" }));

    await waitFor(() => expect(names()[0]).toBe("Иванова Мария"));
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    const create = calls.find((c) => c.cmd === "create_contact")!;
    expect(create.args).toEqual({
      clientId: "c1",
      input: {
        lastName: "Иванова",
        firstName: "Мария",
        middleName: null,
        position: "Бухгалтер",
        note: null,
        isPrimary: true,
      },
    });
    expect(calls.find((c) => c.cmd === "replace_contact_methods")!.args).toEqual({
      owner: { type: "contact", id: "new1" },
      methods: [{ kind: "email", value: "maria@example.com", label: null }],
    });
  });

  it("validates the name and contact methods", async () => {
    render(ContactsPanel, { clientId: "c1" });
    const dialog = await openAdd();
    await userEvent.click(within(dialog).getByRole("button", { name: "Add contact method" }));
    await userEvent.type(within(dialog).getByRole("textbox", { name: "Value" }), "nope");
    await userEvent.click(within(dialog).getByRole("button", { name: "Save" }));
    expect(within(dialog).getByRole("textbox", { name: "First name" })).toHaveAccessibleDescription(
      "This field is required",
    );
    expect(within(dialog).getByRole("textbox", { name: "Value" })).toHaveAccessibleDescription(
      "Invalid email address",
    );
    expect(calls.some((c) => c.cmd === "create_contact")).toBe(false);
  });

  it("edits a contact with its methods", async () => {
    render(ContactsPanel, { clientId: "c1" });
    await waitFor(() => expect(cards()).toHaveLength(2));
    await userEvent.click(within(card("Сидоров")).getByRole("button", { name: "Edit" }));
    const dialog = await screen.findByRole("dialog", { name: "Edit contact" });
    expect(within(dialog).getByRole("textbox", { name: "Value" })).toHaveValue("petr@example.com");
    const position = within(dialog).getByRole("textbox", { name: "Position or role" });
    await userEvent.clear(position);
    await userEvent.type(position, "CEO");
    await userEvent.click(within(dialog).getByRole("button", { name: "Save" }));
    await waitFor(() => expect(within(card("Сидоров")).getByText("CEO")).toBeInTheDocument());
    expect(calls.find((c) => c.cmd === "update_contact")!.args.id).toBe("p1");
  });

  it("makes a contact primary", async () => {
    render(ContactsPanel, { clientId: "c1" });
    await waitFor(() => expect(cards()).toHaveLength(2));
    expect(
      within(card("Сидоров")).queryByRole("button", { name: "Make primary" }),
    ).not.toBeInTheDocument();
    await userEvent.click(within(card("Борисова")).getByRole("button", { name: "Make primary" }));
    await waitFor(() => expect(names()).toEqual(["Борисова Анна", "Сидоров Пётр"]));
  });

  it("deletes after confirmation", async () => {
    render(DialogHost);
    render(ContactsPanel, { clientId: "c1" });
    await waitFor(() => expect(cards()).toHaveLength(2));
    await userEvent.click(within(card("Борисова")).getByRole("button", { name: "Delete" }));
    const dialog = await screen.findByRole("alertdialog", { name: "Delete Борисова Анна?" });
    await userEvent.click(within(dialog).getByRole("button", { name: "Delete" }));
    await waitFor(() => expect(names()).toEqual(["Сидоров Пётр"]));
  });

  it("shows an empty state", async () => {
    contacts = [];
    render(ContactsPanel, { clientId: "c1" });
    expect(await screen.findByText("No contact persons yet")).toBeInTheDocument();
  });

  it("has no a11y violations", async () => {
    const { container } = render(ContactsPanel, { clientId: "c1" });
    await waitFor(() => expect(cards()).toHaveLength(2));
    await expectNoA11yViolations(container);
  });
});
