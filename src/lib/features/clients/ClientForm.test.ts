import { render, screen, waitFor, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type {
  ClientInput_Deserialize as ClientInput,
  Client_Serialize as Client,
  ContactMethodInput_Deserialize as ContactMethodInput,
  ContactMethod_Serialize as ContactMethod,
  EntityRef,
} from "$lib/api/bindings";
import { setAppLocale } from "$lib/i18n/locale.svelte";
import { expectNoA11yViolations } from "../../../test/a11y";
import ClientForm from "./ClientForm.svelte";

interface Call {
  cmd: string;
  args: Record<string, unknown>;
}

let calls: Call[] = [];
let methodsError: unknown = null;

function saved(input: ClientInput, id = "new-id"): Client {
  return {
    ...input,
    id,
    displayName: input.companyName ?? `${input.lastName} ${input.firstName}`,
    tagIds: [],
    createdAt: 0,
    updatedAt: 0,
    archivedAt: null,
  } as Client;
}

beforeEach(() => {
  calls = [];
  methodsError = null;
  mockIPC((cmd, args) => {
    const a = args as Record<string, unknown>;
    calls.push({ cmd, args: a });
    if (cmd === "create_client") return saved(a.input as ClientInput);
    if (cmd === "update_client") return saved(a.input as ClientInput, a.id as string);
    if (cmd === "replace_contact_methods") {
      if (methodsError) throw methodsError;
      return (a.methods as ContactMethodInput[]).map((m, i) => ({ ...m, id: `m${i}` }));
    }
  });
});

afterEach(() => {
  clearMocks();
  setAppLocale("en");
});

const field = (name: string) => screen.getByRole("textbox", { name });
const commands = () => calls.map((c) => c.cmd);

describe("ClientForm", () => {
  it("shows company fields by default and person fields after switching", async () => {
    render(ClientForm, { onsaved: vi.fn() });
    expect(field("Short name")).toBeInTheDocument();
    expect(screen.queryByRole("textbox", { name: "Last name" })).not.toBeInTheDocument();

    await userEvent.click(screen.getByRole("radio", { name: "Person" }));
    expect(field("Last name")).toBeInTheDocument();
    expect(field("First name")).toBeInTheDocument();
    expect(field("Middle name")).toBeInTheDocument();
    expect(screen.queryByRole("textbox", { name: "Short name" })).not.toBeInTheDocument();
  });

  it("shows validation errors in the current language and does not save", async () => {
    render(ClientForm, { onsaved: vi.fn() });
    await userEvent.click(screen.getByRole("button", { name: "Save" }));
    expect(field("Short name")).toHaveAccessibleDescription("This field is required");
    expect(calls).toEqual([]);

    setAppLocale("ru");
    await waitFor(() =>
      expect(field("Краткое название")).toHaveAccessibleDescription("Обязательное поле"),
    );
  });

  it("creates a client with contact methods and reports it", async () => {
    const onsaved = vi.fn();
    render(ClientForm, { onsaved });
    await userEvent.type(field("Short name"), " Ромашка ");
    await userEvent.type(field("Legal name"), "ООО «Ромашка»");
    await userEvent.click(screen.getByRole("combobox", { name: "Status" }));
    await userEvent.click(screen.getByRole("option", { name: "Lead" }));

    await userEvent.click(screen.getByRole("button", { name: "Add contact method" }));
    await userEvent.type(field("Value"), "info@romashka.ru");
    await userEvent.type(field("Label"), "general");
    await userEvent.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(() => expect(onsaved).toHaveBeenCalledOnce());
    expect(commands()).toEqual(["create_client", "replace_contact_methods"]);
    expect(calls[0].args.input).toEqual({
      kind: "company",
      lastName: null,
      firstName: null,
      middleName: null,
      companyName: "Ромашка",
      legalName: "ООО «Ромашка»",
      status: "lead",
      defaultCurrency: null,
      defaultRateMinor: null,
    });
    expect(calls[1].args).toEqual({
      owner: { type: "client", id: "new-id" } satisfies EntityRef,
      methods: [{ kind: "email", value: "info@romashka.ru", label: "general" }],
    });
    expect(onsaved.mock.calls[0][0].id).toBe("new-id");
  });

  it("validates contact methods by type", async () => {
    render(ClientForm, { onsaved: vi.fn() });
    await userEvent.type(field("Short name"), "Ромашка");
    await userEvent.click(screen.getByRole("button", { name: "Add contact method" }));
    await userEvent.type(field("Value"), "not-an-email");
    await userEvent.click(screen.getByRole("button", { name: "Save" }));
    expect(field("Value")).toHaveAccessibleDescription("Invalid email address");

    await userEvent.click(screen.getByRole("combobox", { name: "Type" }));
    await userEvent.click(screen.getByRole("option", { name: "Other" }));
    await userEvent.click(screen.getByRole("button", { name: "Save" }));
    await waitFor(() => expect(commands()).toContain("replace_contact_methods"));
  });

  it("removes contact methods", async () => {
    render(ClientForm, { onsaved: vi.fn() });
    await userEvent.click(screen.getByRole("button", { name: "Add contact method" }));
    await userEvent.click(screen.getByRole("button", { name: "Add contact method" }));
    expect(screen.getAllByRole("textbox", { name: "Value" })).toHaveLength(2);
    await userEvent.click(screen.getByRole("button", { name: "Remove contact method 1" }));
    expect(screen.getAllByRole("textbox", { name: "Value" })).toHaveLength(1);
  });

  it("sends the rate in minor units with the chosen currency", async () => {
    render(ClientForm, { onsaved: vi.fn() });
    await userEvent.type(field("Short name"), "Ромашка");
    await userEvent.click(screen.getByRole("combobox", { name: "Currency" }));
    await userEvent.click(screen.getByRole("option", { name: /^EUR/ }));
    await userEvent.type(field("Hourly rate"), "45.50");
    await userEvent.click(screen.getByRole("button", { name: "Save" }));
    await waitFor(() => expect(commands()).toContain("create_client"));
    const input = calls[0].args.input as ClientInput;
    expect(input.defaultCurrency).toBe("EUR");
    expect(input.defaultRateMinor).toBe(4550);
  });

  it("updates an existing client", async () => {
    const client = saved(
      {
        kind: "person",
        lastName: "Иванов",
        firstName: "Иван",
        middleName: null,
        companyName: null,
        legalName: null,
        status: "active",
        defaultCurrency: null,
        defaultRateMinor: null,
      },
      "c1",
    );
    const methods: ContactMethod[] = [
      { id: "m1", kind: "phone", value: "+7 912 345-67-89", label: null },
    ];
    const onsaved = vi.fn();
    render(ClientForm, { client, methods, onsaved });
    expect(field("Last name")).toHaveValue("Иванов");
    expect(field("Value")).toHaveValue("+7 912 345-67-89");

    await userEvent.clear(field("First name"));
    await userEvent.type(field("First name"), "Пётр");
    await userEvent.click(screen.getByRole("button", { name: "Save" }));
    await waitFor(() => expect(onsaved).toHaveBeenCalledOnce());
    expect(calls[0].cmd).toBe("update_client");
    expect(calls[0].args.id).toBe("c1");
    expect((calls[0].args.input as ClientInput).firstName).toBe("Пётр");
  });

  it("shows backend validation errors under the matching field", async () => {
    methodsError = { code: "validation.phone", params: { field: "value", index: 0 } };
    const onsaved = vi.fn();
    render(ClientForm, { onsaved });
    await userEvent.type(field("Short name"), "Ромашка");
    await userEvent.click(screen.getByRole("button", { name: "Add contact method" }));
    await userEvent.click(screen.getByRole("combobox", { name: "Type" }));
    await userEvent.click(screen.getByRole("option", { name: "Phone" }));
    await userEvent.type(field("Value"), "+7 912 345 67 89");
    await userEvent.click(screen.getByRole("button", { name: "Save" }));
    await waitFor(() => expect(field("Value")).toHaveAccessibleDescription("Invalid phone number"));
    expect(onsaved).not.toHaveBeenCalled();
  });

  it("has no a11y violations", async () => {
    const { container } = render(ClientForm, { onsaved: vi.fn() });
    await userEvent.click(screen.getByRole("button", { name: "Add contact method" }));
    const rows = within(container).getAllByRole("group", { name: "Contact methods" });
    expect(rows).toHaveLength(1);
    await expectNoA11yViolations(container);
  });
});
