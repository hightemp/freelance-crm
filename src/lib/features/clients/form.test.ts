import { describe, expect, it } from "vitest";
import {
  emptyDraft,
  fromClient,
  toInput,
  toMethods,
  validateClient,
  type ClientDraft,
} from "./form";

const company = (patch: Partial<ClientDraft> = {}): ClientDraft => ({
  ...emptyDraft(),
  kind: "company",
  companyName: "Ромашка",
  ...patch,
});

const person = (patch: Partial<ClientDraft> = {}): ClientDraft => ({
  ...emptyDraft(),
  kind: "person",
  lastName: "Иванов",
  ...patch,
});

describe("validateClient", () => {
  it("accepts a minimal company and person", () => {
    expect(validateClient(company())).toEqual({});
    expect(validateClient(person())).toEqual({});
    expect(validateClient(person({ lastName: "", firstName: "Иван" }))).toEqual({});
  });

  it("requires a company name for companies", () => {
    expect(validateClient(company({ companyName: "  " }))).toEqual({
      companyName: "validation.required",
    });
  });

  it("requires last or first name for persons", () => {
    expect(validateClient(person({ lastName: "", middleName: "Иванович" }))).toEqual({
      firstName: "validation.required",
    });
  });

  it("ignores fields of the other kind", () => {
    expect(validateClient(company({ lastName: "" }))).toEqual({});
    expect(validateClient(person({ companyName: "" }))).toEqual({});
  });

  it("rejects a negative rate", () => {
    expect(validateClient(company({ rateMinor: -1 }))).toEqual({ rateMinor: "validation.min" });
  });

  it.each([
    ["email", "ivan@example.com", undefined],
    ["email", "ivan@", "validation.email"],
    ["phone", "+7 (912) 345-67-89", undefined],
    ["phone", "12-34", "validation.phone"],
    ["whatsapp", "+79123456789", undefined],
    ["whatsapp", "@ivan", "validation.phone"],
    ["telegram", "@ivan_petrov", undefined],
    ["telegram", "https://t.me/ivan_petrov", undefined],
    ["telegram", "+7 912 345 67 89", undefined],
    ["telegram", "@abc", "validation.telegram"],
    ["website", "example.com", undefined],
    ["website", "https://example.com/path", undefined],
    ["website", "not a url", "validation.url"],
    ["other", "skype: ivan", undefined],
    ["other", "  ", "validation.required"],
  ] as const)("validates %s value %j", (kind, value, error) => {
    const errors = validateClient(company({ methods: [{ kind, value, label: "" }] }));
    expect(errors["methods.0.value"]).toBe(error);
  });

  it("reports the index of the bad method", () => {
    const errors = validateClient(
      company({
        methods: [
          { kind: "email", value: "ok@example.com", label: "" },
          { kind: "email", value: "bad", label: "" },
        ],
      }),
    );
    expect(errors).toEqual({ "methods.1.value": "validation.email" });
  });
});

describe("DTO mapping", () => {
  it("builds a trimmed ClientInput, dropping fields of the other kind", () => {
    const draft = company({
      companyName: " Ромашка ",
      legalName: "",
      lastName: "Иванов",
      status: "lead",
      currency: "RUB",
      rateMinor: 250_000,
    });
    expect(toInput(draft)).toEqual({
      kind: "company",
      lastName: null,
      firstName: null,
      middleName: null,
      companyName: "Ромашка",
      legalName: null,
      status: "lead",
      defaultCurrency: "RUB",
      defaultRateMinor: 250_000,
    });
  });

  it("maps contact methods with empty labels to null", () => {
    expect(
      toMethods(person({ methods: [{ kind: "email", value: " a@b.co ", label: " " }] })),
    ).toEqual([{ kind: "email", value: "a@b.co", label: null }]);
  });

  it("round-trips an existing client", () => {
    const draft = fromClient(
      {
        id: "1",
        kind: "person",
        lastName: "Иванов",
        firstName: "Иван",
        middleName: null,
        companyName: null,
        legalName: null,
        status: "paused",
        defaultCurrency: "EUR",
        defaultRateMinor: 5000,
        displayName: "Иванов Иван",
        tagIds: [],
        createdAt: 0,
        updatedAt: 0,
        archivedAt: null,
      },
      [{ id: "m1", kind: "phone", value: "+7 912", label: null }],
    );
    expect(draft).toMatchObject({
      kind: "person",
      lastName: "Иванов",
      middleName: "",
      currency: "EUR",
      rateMinor: 5000,
      methods: [{ kind: "phone", value: "+7 912", label: "" }],
    });
    expect(toInput(draft).status).toBe("paused");
  });
});
