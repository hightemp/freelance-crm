import type { SecretKind } from "$lib/api/bindings";
import { m } from "$lib/paraglide/messages";

type MessageFn = () => string;
const messages = m as unknown as Record<string, MessageFn | undefined>;

/** Label of a secret kind (`secret_kind_<kind>`). */
export function kindLabel(kind: SecretKind): string {
  return messages[`secret_kind_${kind}`]?.() ?? kind;
}

/**
 * Label of a template field (`secret_field_<key>`); a Rust test keeps the
 * catalogs complete for every template.
 */
export function fieldLabel(key: string): string {
  return messages[`secret_field_${key}`]?.() ?? key;
}
