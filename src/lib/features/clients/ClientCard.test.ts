import { cleanup, render, screen, waitFor, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { clearMocks, mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Client_Serialize as Client, Tag } from "$lib/api/bindings";
import { expectNoA11yViolations } from "../../../test/a11y";
import { typeInEditor } from "../../../test/editor";
import ClientCard from "./ClientCard.svelte";

const goto = vi.hoisted(() => vi.fn());
vi.mock("$app/navigation", () => ({ goto }));

const T0 = Date.UTC(2024, 0, 15);

function company(patch: Partial<Client> = {}): Client {
  return {
    id: "c1",
    kind: "company",
    lastName: null,
    firstName: null,
    middleName: null,
    companyName: "Ромашка",
    legalName: "ООО «Ромашка»",
    status: "active",
    defaultCurrency: "RUB",
    defaultRateMinor: 250_000,
    displayName: "Ромашка",
    tagIds: ["t1"],
    createdAt: T0,
    updatedAt: T0,
    archivedAt: null,
    ...patch,
  };
}

let client: Client;
let invoices = 0;
let tags: Tag[];
let calls: { cmd: string; args: Record<string, unknown> }[];

beforeEach(() => {
  client = company();
  tags = [
    { id: "t1", name: "VIP", color: "#ef4444", createdAt: 0 },
    { id: "t2", name: "Срочно", color: "#3b82f6", createdAt: 0 },
  ];
  calls = [];
  mockWindows("main");
  mockIPC(
    (cmd, args) => {
      const a = (args ?? {}) as Record<string, unknown>;
      calls.push({ cmd, args: a });
      switch (cmd) {
        case "get_client":
          if (a.id !== client.id) throw { code: "client.not_found", params: { id: a.id } };
          return client;
        case "list_contact_methods":
          return [{ id: "m1", kind: "phone", value: "+7 495 123-45-67", label: "office" }];
        case "list_tags":
          return tags;
        case "create_tag": {
          const tag = { id: "t-new", ...(a.input as object), createdAt: 0 } as Tag;
          tags = [...tags, tag];
          return tag;
        }
        case "set_tags": {
          const ids = a.tagIds as string[];
          client = { ...client, tagIds: ids };
          return tags.filter((t) => ids.includes(t.id));
        }
        case "list_country_schemas":
          return [];
        case "get_requisites":
          return null;
        case "list_projects":
        case "list_invoices":
        case "list_clients":
        case "list_tasks":
        case "list_board_columns":
        case "list_secrets":
        case "list_attachments":
          return [];
        case "interaction_feed":
          return { items: [], next: null };
        case "vault_status":
          return "locked";
        case "list_contacts":
        case "list_notes":
        case "list_custom_fields":
          return [];
        case "create_contact":
          return { id: "p1", clientId: "c1", ...(a.input as object), displayName: "X" };
        case "replace_contact_methods":
          return [];
        case "create_note":
          return { id: "n1", ...(a.input as object), createdAt: 0, updatedAt: 0 };
        case "create_custom_field":
          return { id: "f1", ...(a.input as object) };
        case "archive_client":
          client = { ...client, archivedAt: T0 + 1 };
          return client;
        case "restore_client":
          client = { ...client, archivedAt: null };
          return client;
        case "client_delete_impact":
          return {
            contacts: 2,
            contactMethods: 3,
            customFields: 0,
            notes: 1,
            events: 0,
            timeEntries: 0,
            interactions: 0,
            invoices,
          };
        case "delete_client":
          return null;
      }
    },
    { shouldMockEvents: true },
  );
});

afterEach(() => {
  // Unmount first: components unsubscribe from mocked events on destroy.
  cleanup();
  clearMocks();
  goto.mockReset();
});

const tab = (name: string) => screen.getByRole("tab", { name });
const commands = () => calls.map((c) => c.cmd);

describe("ClientCard", () => {
  it("shows the header and overview", async () => {
    render(ClientCard, { id: "c1" });
    expect(await screen.findByRole("heading", { level: 1, name: "Ромашка" })).toBeInTheDocument();
    expect(screen.getByText("Active")).toBeInTheDocument();
    const overview = screen.getByRole("tabpanel");
    expect(within(overview).getByText("ООО «Ромашка»")).toBeInTheDocument();
    expect(within(overview).getByText("+7 495 123-45-67")).toBeInTheDocument();
    expect(within(overview).getByText(/2,500\.00/)).toBeInTheDocument();
    expect(await screen.findByText("VIP")).toBeInTheDocument();
  });

  it("lists the client's projects and creates one for this client", async () => {
    render(ClientCard, { id: "c1" });
    await userEvent.click(await screen.findByRole("tab", { name: "Projects" }));
    expect(await screen.findByText("No projects yet")).toBeInTheDocument();
    expect(calls.find((c) => c.cmd === "list_projects")!.args.filter).toMatchObject({
      clientId: "c1",
    });
    await userEvent.click(screen.getByRole("button", { name: "New project" }));
    expect(goto).toHaveBeenCalledWith("/projects/new?client=c1");
  });

  it("lists the client's tasks and creates one for this client", async () => {
    render(ClientCard, { id: "c1" });
    await userEvent.click(await screen.findByRole("tab", { name: "Tasks" }));
    expect(await screen.findByText("No tasks yet")).toBeInTheDocument();
    expect(calls.find((c) => c.cmd === "list_tasks")!.args.filter).toMatchObject({
      scope: { type: "client", id: "c1" },
    });
    await userEvent.click(screen.getByRole("button", { name: "New task" }));
    expect(goto).toHaveBeenCalledWith("/tasks/new?client=c1");
  });

  it("opens on the finance tab with the client's invoices", async () => {
    render(ClientCard, { id: "c1", initialTab: "finance" });
    expect(await screen.findByRole("tab", { name: "Finance" })).toHaveAttribute(
      "aria-selected",
      "true",
    );
    expect(await screen.findByText("No invoices yet")).toBeInTheDocument();
    expect(calls.find((c) => c.cmd === "list_invoices")!.args).toEqual({
      clientId: "c1",
      projectId: null,
    });
  });

  it("shows the client's history", async () => {
    render(ClientCard, { id: "c1" });
    await userEvent.click(await screen.findByRole("tab", { name: "History" }));
    expect(await screen.findByText("No records yet")).toBeInTheDocument();
    expect(calls.find((c) => c.cmd === "interaction_feed")!.args.clientId).toBe("c1");
  });

  it("shows the client's files", async () => {
    render(ClientCard, { id: "c1" });
    await userEvent.click(await screen.findByRole("tab", { name: "Files" }));
    expect(await screen.findByText("No files yet")).toBeInTheDocument();
    expect(calls.find((c) => c.cmd === "list_attachments")!.args.owner).toEqual({
      type: "client",
      id: "c1",
    });
  });

  it("asks to unlock the vault before showing secrets", async () => {
    render(ClientCard, { id: "c1" });
    await userEvent.click(await screen.findByRole("tab", { name: "Secrets" }));
    expect(await screen.findByRole("button", { name: "Unlock" })).toBeInTheDocument();
    expect(commands()).not.toContain("list_secrets");
  });

  it("switches tabs; unfinished sections are placeholders", async () => {
    render(ClientCard, { id: "c1" });
    await userEvent.click(await screen.findByRole("tab", { name: "Notes" }));
    expect(tab("Notes")).toHaveAttribute("aria-selected", "true");
    expect(await screen.findByRole("textbox", { name: "New note" })).toBeInTheDocument();

    await userEvent.click(tab("Events"));
    expect(screen.getByRole("tabpanel")).toHaveTextContent("This section is coming soon.");
  });

  it("has a requisites tab", async () => {
    render(ClientCard, { id: "c1" });
    await userEvent.click(await screen.findByRole("tab", { name: "Requisites" }));
    await waitFor(() => expect(commands()).toContain("get_requisites"));
    expect(calls.find((c) => c.cmd === "get_requisites")!.args.owner).toEqual({
      type: "client",
      id: "c1",
    });
  });

  it("has no contacts tab for a person", async () => {
    client = company({ kind: "person", lastName: "Иванов", companyName: null });
    render(ClientCard, { id: "c1" });
    await screen.findByRole("tab", { name: "Notes" });
    expect(screen.queryByRole("tab", { name: "Contacts" })).not.toBeInTheDocument();
  });

  it("adds a contact, a note and a field from the card", async () => {
    render(ClientCard, { id: "c1" });

    await userEvent.click(await screen.findByRole("tab", { name: "Contacts" }));
    await userEvent.click(await screen.findByRole("button", { name: "Add contact" }));
    const contactDialog = await screen.findByRole("dialog", { name: "New contact" });
    await userEvent.type(
      within(contactDialog).getByRole("textbox", { name: "First name" }),
      "Анна",
    );
    await userEvent.click(within(contactDialog).getByRole("button", { name: "Save" }));
    await waitFor(() => expect(commands()).toContain("create_contact"));

    await userEvent.click(tab("Notes"));
    typeInEditor(await screen.findByRole("textbox", { name: "New note" }), "Привет");
    await userEvent.click(screen.getByRole("button", { name: "Add note" }));
    await waitFor(() => expect(commands()).toContain("create_note"));
    expect(calls.find((c) => c.cmd === "create_note")!.args.owner).toEqual({
      type: "client",
      id: "c1",
    });

    await userEvent.click(tab("Fields"));
    await userEvent.click(await screen.findByRole("button", { name: "Add field" }));
    const fieldDialog = await screen.findByRole("dialog", { name: "New field" });
    await userEvent.type(within(fieldDialog).getByRole("textbox", { name: "Name" }), "ИНН");
    await userEvent.click(within(fieldDialog).getByRole("button", { name: "Save" }));
    await waitFor(() => expect(commands()).toContain("create_custom_field"));
  });

  it("attaches existing and new tags", async () => {
    render(ClientCard, { id: "c1" });
    const input = await screen.findByRole("combobox", { name: "Tags" });
    await userEvent.type(input, "Срочно{Enter}");
    await waitFor(() =>
      expect(calls.findLast((c) => c.cmd === "set_tags")?.args.tagIds).toEqual(["t1", "t2"]),
    );
    expect(commands()).not.toContain("create_tag");

    await userEvent.type(input, "Новый{Enter}");
    await waitFor(() =>
      expect(calls.findLast((c) => c.cmd === "set_tags")?.args.tagIds).toEqual([
        "t1",
        "t2",
        "t-new",
      ]),
    );
    expect((calls.find((c) => c.cmd === "create_tag")!.args.input as Tag).name).toBe("Новый");
  });

  it("opens the edit page", async () => {
    render(ClientCard, { id: "c1" });
    await userEvent.click(await screen.findByRole("button", { name: "Edit" }));
    expect(goto).toHaveBeenCalledWith("/clients/c1/edit");
  });

  it("shows an error for an unknown client", async () => {
    render(ClientCard, { id: "missing" });
    expect(await screen.findByText("Client not found")).toBeInTheDocument();
  });

  it("has no a11y violations", async () => {
    const { container } = render(ClientCard, { id: "c1" });
    await screen.findByRole("heading", { level: 1 });
    await expectNoA11yViolations(container);
  });
});

describe("ClientCard archive and delete", () => {
  async function menu(item: string) {
    await userEvent.click(await screen.findByRole("button", { name: "More actions" }));
    await userEvent.click(screen.getByRole("menuitem", { name: item }));
  }

  it("archives and restores", async () => {
    render(ClientCard, { id: "c1" });
    await menu("Archive");
    expect(await screen.findByText("Archived")).toBeInTheDocument();
    await menu("Restore from archive");
    await waitFor(() => expect(screen.queryByText("Archived")).not.toBeInTheDocument());
    expect(commands()).toEqual(expect.arrayContaining(["archive_client", "restore_client"]));
  });

  it("shows what will be deleted and deletes after confirmation", async () => {
    render(ClientCard, { id: "c1" });
    await menu("Delete client");
    const dialog = await screen.findByRole("alertdialog", { name: "Delete Ромашка?" });
    const items = within(dialog)
      .getAllByRole("listitem")
      .map((li) => li.textContent?.replace(/\s+/g, " ").trim());
    expect(items).toEqual(["Contacts 2", "Contact methods 3", "Notes 1"]);
    await userEvent.click(within(dialog).getByRole("button", { name: "Delete" }));
    await waitFor(() => expect(goto).toHaveBeenCalledWith("/clients"));
    expect(calls.find((c) => c.cmd === "delete_client")?.args.id).toBe("c1");
  });

  it("offers archiving instead of deleting", async () => {
    render(ClientCard, { id: "c1" });
    await menu("Delete client");
    const dialog = await screen.findByRole("alertdialog");
    await userEvent.click(within(dialog).getByRole("button", { name: "Archive instead" }));
    expect(await screen.findByText("Archived")).toBeInTheDocument();
    expect(commands()).not.toContain("delete_client");
    expect(screen.queryByRole("alertdialog")).not.toBeInTheDocument();
  });

  it("does not delete clients with invoices", async () => {
    invoices = 2;
    render(ClientCard, { id: "c1" });
    await menu("Delete client");
    const dialog = await screen.findByRole("alertdialog");
    expect(within(dialog).getByRole("alert")).toHaveTextContent("2 invoices");
    expect(within(dialog).queryByRole("button", { name: "Delete" })).not.toBeInTheDocument();
    expect(within(dialog).getByRole("button", { name: "Archive instead" })).toBeInTheDocument();
    invoices = 0;
  });

  it("can be cancelled", async () => {
    render(ClientCard, { id: "c1" });
    await menu("Delete client");
    const dialog = await screen.findByRole("alertdialog");
    await userEvent.click(within(dialog).getByRole("button", { name: "Cancel" }));
    expect(commands()).not.toContain("delete_client");
    expect(goto).not.toHaveBeenCalled();
  });
});
