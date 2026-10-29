import { render, screen, waitFor, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import type {
  CountrySchema_Serialize as CountrySchema,
  RequisitesInput_Deserialize as RequisitesInput,
  RequisitesOwner,
  Requisites_Serialize as Requisites,
} from "$lib/api/bindings";
import { setAppLocale } from "$lib/i18n/locale.svelte";
import DialogHost from "$lib/ui/DialogHost.svelte";
import { resetDialogs } from "$lib/ui/dialogs.svelte";
import { expectNoA11yViolations } from "../../../test/a11y";
import RequisitesForm from "./RequisitesForm.svelte";
import { resetSchemas } from "./schemas";

const L = (en: string, ru: string) => ({ en, ru });

const field = (key: string, group: string, labels: Record<string, string>, extra = {}) => ({
  key,
  group,
  kind: "text" as const,
  input: "text" as const,
  maxLength: null,
  validator: null,
  options: [],
  labels,
  ...extra,
});

const SCHEMAS: CountrySchema[] = [
  {
    code: "RU",
    labels: L("Russia", "Россия"),
    invoiceTemplate: "ru_invoice",
    groups: [
      { key: "main", labels: L("Main", "Основные") },
      { key: "address", labels: L("Address", "Адрес") },
    ],
    fields: [
      field("inn", "main", L("Taxpayer ID (INN)", "ИНН"), { input: "digits", maxLength: 12 }),
      field("kpp", "main", L("KPP", "КПП"), { input: "upper", maxLength: 9 }),
      field("address", "address", L("Registered address", "Адрес регистрации"), {
        kind: "textarea",
      }),
    ],
    legalForms: [
      {
        key: "self_employed",
        labels: L("Self-employed (NPD)", "Самозанятый (НПД)"),
        fields: [
          { key: "inn", required: true, validator: null },
          { key: "address", required: false, validator: null },
        ],
      },
      {
        key: "ooo",
        labels: L("LLC (OOO)", "ООО"),
        fields: [
          { key: "inn", required: true, validator: null },
          { key: "kpp", required: true, validator: null },
          { key: "address", required: true, validator: null },
        ],
      },
    ],
    taxPresets: [],
  },
  {
    code: "US",
    labels: L("United States", "США"),
    invoiceTemplate: "us_invoice",
    groups: [{ key: "address", labels: L("Address", "Адрес") }],
    fields: [
      field("state", "address", L("State", "Штат"), {
        kind: "select",
        options: [
          { value: "CA", labels: L("California", "Калифорния") },
          { value: "NY", labels: L("New York", "Нью-Йорк") },
        ],
      }),
    ],
    legalForms: [
      {
        key: "llc",
        labels: L("LLC", "LLC"),
        fields: [{ key: "state", required: true, validator: null }],
      },
    ],
    taxPresets: [],
  },
  {
    code: "INTL",
    labels: L("International", "Международные"),
    invoiceTemplate: "international",
    groups: [
      { key: "address", labels: L("Address", "Адрес") },
      { key: "bank", labels: L("Bank", "Банк") },
    ],
    fields: [
      field("address", "address", L("Address", "Адрес"), { kind: "textarea" }),
      field("iban", "bank", L("IBAN", "IBAN"), { input: "upper", maxLength: 34 }),
    ],
    legalForms: [
      {
        key: "company",
        labels: L("Company", "Компания"),
        fields: [
          { key: "address", required: true, validator: null },
          { key: "iban", required: false, validator: null },
        ],
      },
    ],
    taxPresets: [],
  },
];

const PROFILE: RequisitesOwner = { type: "profile" };
let stored: Requisites | null = null;
let saves: RequisitesInput[] = [];
let response: unknown = null;

beforeEach(() => {
  resetSchemas();
  stored = null;
  saves = [];
  response = null;
  mockIPC((cmd, args) => {
    const a = args as Record<string, unknown>;
    switch (cmd) {
      case "list_country_schemas":
        return SCHEMAS;
      case "get_requisites":
        return stored;
      case "save_requisites": {
        const input = a.input as RequisitesInput;
        saves.push(input);
        if (response) return response;
        stored = { ...input, name: input.name ?? null, updatedAt: 1 } as Requisites;
        return { status: "saved", requisites: stored };
      }
    }
  });
});

afterEach(() => {
  clearMocks();
  resetDialogs();
  setAppLocale("en");
});

const box = (name: string) => screen.getByRole("textbox", { name });
const queryBox = (name: string) => screen.queryByRole("textbox", { name });

async function choose(label: string, option: string) {
  await userEvent.click(screen.getByRole("combobox", { name: label }));
  await userEvent.click(screen.getByRole("option", { name: option }));
}

describe("RequisitesForm", () => {
  it("starts with the first country and form when nothing is saved", async () => {
    render(RequisitesForm, { owner: PROFILE });
    expect(await screen.findByRole("combobox", { name: "Country" })).toHaveTextContent("Russia");
    expect(screen.getByRole("combobox", { name: "Legal form" })).toHaveTextContent(
      "Self-employed (NPD)",
    );
    expect(box("Taxpayer ID (INN)")).toBeRequired();
    expect(queryBox("KPP")).not.toBeInTheDocument();
  });

  it("loads saved requisites", async () => {
    stored = {
      country: "RU",
      legalForm: "ooo",
      name: null,
      values: { inn: "7707083893", kpp: "773601001", address: "Москва" },
      updatedAt: 1,
    };
    render(RequisitesForm, { owner: PROFILE });
    await waitFor(() => expect(box("KPP")).toHaveValue("773601001"));
    expect(box("Taxpayer ID (INN)")).toHaveValue("7707083893");
    expect(box("Registered address")).toHaveValue("Москва");
  });

  it("rebuilds the fields when the legal form changes", async () => {
    render(RequisitesForm, { owner: PROFILE });
    await screen.findByRole("combobox", { name: "Legal form" });
    await choose("Legal form", "LLC (OOO)");
    expect(box("KPP")).toBeRequired();
    expect(box("Registered address")).toBeRequired();
    expect(screen.getByRole("group", { name: "Main" })).toBeInTheDocument();
  });

  it("asks before changing the country and clears fields it does not have", async () => {
    render(DialogHost);
    render(RequisitesForm, { owner: PROFILE });
    await screen.findByRole("combobox", { name: "Country" });
    await userEvent.type(box("Taxpayer ID (INN)"), "500100732259");
    await userEvent.type(box("Registered address"), "Москва");

    await choose("Country", "International");
    let dialog = await screen.findByRole("alertdialog", { name: "Change country?" });
    await userEvent.click(within(dialog).getByRole("button", { name: "Cancel" }));
    expect(screen.getByRole("combobox", { name: "Country" })).toHaveTextContent("Russia");
    expect(box("Taxpayer ID (INN)")).toHaveValue("500100732259");

    await choose("Country", "International");
    dialog = await screen.findByRole("alertdialog", { name: "Change country?" });
    await userEvent.click(within(dialog).getByRole("button", { name: "Change country" }));
    await waitFor(() => expect(box("IBAN")).toBeInTheDocument());
    expect(queryBox("Taxpayer ID (INN)")).not.toBeInTheDocument();
    expect(box("Address")).toHaveValue("Москва");
  });

  it("does not ask when there is nothing to lose", async () => {
    render(DialogHost);
    render(RequisitesForm, { owner: PROFILE });
    await screen.findByRole("combobox", { name: "Country" });
    await choose("Country", "International");
    expect(await screen.findByRole("textbox", { name: "IBAN" })).toBeInTheDocument();
    expect(screen.queryByRole("alertdialog")).not.toBeInTheDocument();
  });

  it("masks the INN to digits and its maximum length", async () => {
    render(RequisitesForm, { owner: PROFILE });
    await screen.findByRole("combobox", { name: "Country" });
    await userEvent.type(box("Taxpayer ID (INN)"), "50-01a0073 225999");
    expect(box("Taxpayer ID (INN)")).toHaveValue("500100732259");
  });

  it("saves the form and confirms the country change to the backend", async () => {
    stored = {
      country: "RU",
      legalForm: "self_employed",
      name: "Иванов",
      values: { address: "Москва" },
      updatedAt: 1,
    };
    render(DialogHost);
    render(RequisitesForm, { owner: PROFILE, withName: true });
    await waitFor(() => expect(box("Name on documents")).toHaveValue("Иванов"));
    // Only the address is filled, and INTL has it too: nothing to confirm in
    // the UI, but the backend still gets the explicit country change flag.
    await choose("Country", "International");
    await userEvent.type(await screen.findByRole("textbox", { name: "IBAN" }), "de89 3704");
    await userEvent.click(screen.getByRole("button", { name: "Save" }));
    await waitFor(() => expect(saves).toHaveLength(1));
    expect(saves[0]).toEqual({
      country: "INTL",
      legalForm: "company",
      name: "Иванов",
      values: { address: "Москва", iban: "DE89 3704" },
      confirmCountryChange: true,
    });
  });

  it("shows validator errors under the field in the current language", async () => {
    response = {
      status: "invalid",
      errors: [
        { field: "inn", rule: "ru_inn_12" },
        { field: "address", rule: "required" },
      ],
    };
    render(RequisitesForm, { owner: PROFILE });
    await screen.findByRole("combobox", { name: "Country" });
    await userEvent.type(box("Taxpayer ID (INN)"), "123");
    await userEvent.click(screen.getByRole("button", { name: "Save" }));
    await waitFor(() =>
      expect(box("Taxpayer ID (INN)")).toHaveAccessibleDescription(
        "An individual's INN has 12 digits; check the number",
      ),
    );
    expect(box("Registered address")).toHaveAccessibleDescription("This field is required");

    setAppLocale("ru");
    await waitFor(() =>
      expect(box("ИНН")).toHaveAccessibleDescription(
        "ИНН физлица или ИП состоит из 12 цифр; проверьте номер",
      ),
    );
  });

  it("offers list fields as a select", async () => {
    render(RequisitesForm, { owner: PROFILE });
    await screen.findByRole("combobox", { name: "Country" });
    await choose("Country", "United States");
    await choose("State", "California");
    await userEvent.click(screen.getByRole("button", { name: "Save" }));
    await waitFor(() => expect(saves).toHaveLength(1));
    expect(saves[0].values).toEqual({ state: "CA" });

    setAppLocale("ru");
    await waitFor(() =>
      expect(screen.getByRole("combobox", { name: "Штат" })).toHaveTextContent("Калифорния"),
    );
  });

  it("shows the name field only for the profile", async () => {
    render(RequisitesForm, { owner: { type: "client", id: "c1" } });
    await screen.findByRole("combobox", { name: "Country" });
    expect(queryBox("Name on documents")).not.toBeInTheDocument();
  });

  it("has no a11y violations", async () => {
    const { container } = render(RequisitesForm, { owner: PROFILE, withName: true });
    await screen.findByRole("combobox", { name: "Country" });
    await expectNoA11yViolations(container);
  });
});
