// Contact method kinds, their client-side validation (mirroring crm_core)
// and conversion between drafts and DTOs.

import type {
  ContactMethodInput_Deserialize as ContactMethodInput,
  ContactMethodKind,
  ContactMethod_Serialize as ContactMethod,
} from "$lib/api/bindings";
import { m } from "$lib/paraglide/messages";

export interface MethodDraft {
  kind: ContactMethodKind;
  value: string;
  label: string;
}

export const METHOD_KINDS: readonly ContactMethodKind[] = [
  "email",
  "phone",
  "telegram",
  "whatsapp",
  "website",
  "other",
];

export function methodLabel(kind: ContactMethodKind): string {
  switch (kind) {
    case "email":
      return m.contact_method_email();
    case "phone":
      return m.contact_method_phone();
    case "telegram":
      return m.contact_method_telegram();
    case "whatsapp":
      return m.contact_method_whatsapp();
    case "website":
      return m.contact_method_website();
    case "other":
      return m.contact_method_other();
  }
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
export function methodError(kind: ContactMethodKind, raw: string): string | undefined {
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

export function fromMethods(methods: ContactMethod[]): MethodDraft[] {
  return methods.map((m) => ({ kind: m.kind, value: m.value, label: m.label ?? "" }));
}

export function toMethodInputs(drafts: MethodDraft[]): ContactMethodInput[] {
  return drafts.map((d) => ({
    kind: d.kind,
    value: d.value.trim(),
    label: d.label.trim() === "" ? null : d.label.trim(),
  }));
}
