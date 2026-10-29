// Client form model: a string-based draft for inputs, its validation with
// zod and the mapping to command DTOs. Validation mirrors crm_core rules so
// most mistakes are shown before saving; the backend stays authoritative.
// Errors are codes ("validation.email"), translated when rendered.

import { z } from "zod";
import type {
  ClientInput_Deserialize as ClientInput,
  ClientKind,
  ClientStatus,
  Client_Serialize as Client,
  ContactMethodInput_Deserialize as ContactMethodInput,
  ContactMethodKind,
  ContactMethod,
} from "$lib/api/bindings";

export interface MethodDraft {
  kind: ContactMethodKind;
  value: string;
  label: string;
}

export interface ClientDraft {
  kind: ClientKind;
  lastName: string;
  firstName: string;
  middleName: string;
  companyName: string;
  legalName: string;
  status: ClientStatus;
  /** `null` uses the app default currency. */
  currency: string | null;
  rateMinor: number | null;
  methods: MethodDraft[];
}

/** Field path ("companyName", "methods.1.value") → error code. */
export type FormErrors = Record<string, string>;

export function emptyDraft(): ClientDraft {
  return {
    kind: "company",
    lastName: "",
    firstName: "",
    middleName: "",
    companyName: "",
    legalName: "",
    status: "active",
    currency: null,
    rateMinor: null,
    methods: [],
  };
}

export function fromClient(client: Client, methods: ContactMethod[]): ClientDraft {
  return {
    kind: client.kind,
    lastName: client.lastName ?? "",
    firstName: client.firstName ?? "",
    middleName: client.middleName ?? "",
    companyName: client.companyName ?? "",
    legalName: client.legalName ?? "",
    status: client.status,
    currency: client.defaultCurrency,
    rateMinor: client.defaultRateMinor,
    methods: methods.map((m) => ({ kind: m.kind, value: m.value, label: m.label ?? "" })),
  };
}

const EMAIL = /^[^\s@]+@[^\s@.]+(\.[^\s@.]+)+$/;
const PHONE_CHARS = /^\+?[\d\s().-]+$/;
const TELEGRAM_NAME = /^(?:https?:\/\/)?(?:t\.me\/)?@?[A-Za-z][A-Za-z0-9_]{4,31}$/;
const WEBSITE = /^(?:https?:\/\/)?[^\s/.]+(\.[^\s/.]+)+(\/\S*)?$/i;

function isPhone(value: string): boolean {
  const digits = value.replace(/\D/g, "").length;
  return PHONE_CHARS.test(value) && digits >= 5 && digits <= 15;
}

/** Error code for a contact method value, or `undefined` if it is valid. */
function methodError(kind: ContactMethodKind, raw: string): string | undefined {
  const value = raw.trim();
  if (value === "") return "validation.required";
  switch (kind) {
    case "email":
      return EMAIL.test(value) ? undefined : "validation.email";
    case "phone":
    case "whatsapp":
      return isPhone(value) ? undefined : "validation.phone";
    case "telegram":
      return /^[+\d]/.test(value)
        ? isPhone(value)
          ? undefined
          : "validation.phone"
        : TELEGRAM_NAME.test(value)
          ? undefined
          : "validation.telegram";
    case "website":
      return WEBSITE.test(value) ? undefined : "validation.url";
    case "other":
      return undefined;
  }
}

const blank = (s: string) => s.trim() === "";

const schema = z
  .object({
    kind: z.enum(["person", "company"]),
    lastName: z.string(),
    firstName: z.string(),
    companyName: z.string(),
    rateMinor: z.number().int().min(0, "validation.min").nullable(),
    methods: z.array(z.object({ kind: z.string(), value: z.string() })),
  })
  .superRefine((d, ctx) => {
    if (d.kind === "person" && blank(d.lastName) && blank(d.firstName)) {
      ctx.addIssue({ code: "custom", path: ["firstName"], message: "validation.required" });
    }
    if (d.kind === "company" && blank(d.companyName)) {
      ctx.addIssue({ code: "custom", path: ["companyName"], message: "validation.required" });
    }
    d.methods.forEach((m, i) => {
      const error = methodError(m.kind as ContactMethodKind, m.value);
      if (error) ctx.addIssue({ code: "custom", path: ["methods", i, "value"], message: error });
    });
  });

export function validateClient(draft: ClientDraft): FormErrors {
  const result = schema.safeParse(draft);
  if (result.success) return {};
  const errors: FormErrors = {};
  for (const issue of result.error.issues) {
    errors[issue.path.join(".")] ??= issue.message;
  }
  return errors;
}

const text = (s: string) => (blank(s) ? null : s.trim());

export function toInput(d: ClientDraft): ClientInput {
  const person = d.kind === "person";
  return {
    kind: d.kind,
    lastName: person ? text(d.lastName) : null,
    firstName: person ? text(d.firstName) : null,
    middleName: person ? text(d.middleName) : null,
    companyName: person ? null : text(d.companyName),
    legalName: person ? null : text(d.legalName),
    status: d.status,
    defaultCurrency: d.currency,
    defaultRateMinor: d.rateMinor,
  };
}

export function toMethods(d: ClientDraft): ContactMethodInput[] {
  return d.methods.map((m) => ({ kind: m.kind, value: m.value.trim(), label: text(m.label) }));
}
