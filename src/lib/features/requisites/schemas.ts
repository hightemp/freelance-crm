import {
  commands,
  type CountrySchema_Serialize as CountrySchema,
  type FieldDef_Serialize as FieldDef,
} from "$lib/api/bindings";
import { i18n } from "$lib/i18n/locale.svelte";

let cache: Promise<CountrySchema[]> | null = null;

/** Built-in country schemas; they never change while the app runs. */
export function loadSchemas(): Promise<CountrySchema[]> {
  cache ??= commands.listCountrySchemas();
  return cache;
}

/** Forgets the cache (tests). */
export function resetSchemas(): void {
  cache = null;
}

/** Label in the current language, falling back to English. */
export function label(labels: Partial<Record<string, string>>): string {
  return labels[i18n.locale] ?? labels.en ?? "";
}

/** Input mask: digits-only fields drop everything else, codes are upper-cased. */
export function mask(def: FieldDef, raw: string): string {
  switch (def.input) {
    case "digits": {
      const digits = raw.replace(/\D/g, "");
      return def.maxLength === null ? digits : digits.slice(0, def.maxLength);
    }
    case "upper":
      return raw.toUpperCase();
    case "text":
      return raw;
  }
}
