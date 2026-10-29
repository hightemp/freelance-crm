import { render, screen, waitFor, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
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
let tags: Tag[];
let calls: { cmd: string; args: Record<string, unknown> }[];

beforeEach(() => {
  client = company();
  tags = [
    { id: "t1", name: "VIP", color: "#ef4444", createdAt: 0 },
    { id: "t2", name: "Срочно", color: "#3b82f6", createdAt: 0 },
  ];
  calls = [];
  mockIPC((cmd, args) => {
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
    }
  });
});

afterEach(() => {
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
    expect(screen.getByText("VIP")).toBeInTheDocument();
  });

  it("switches tabs; unfinished sections are placeholders", async () => {
    render(ClientCard, { id: "c1" });
    await userEvent.click(await screen.findByRole("tab", { name: "Notes" }));
    expect(tab("Notes")).toHaveAttribute("aria-selected", "true");
    expect(await screen.findByRole("textbox", { name: "New note" })).toBeInTheDocument();

    await userEvent.click(tab("Projects"));
    expect(screen.getByRole("tabpanel")).toHaveTextContent("This section is coming soon.");
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
