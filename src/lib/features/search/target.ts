import { resolve } from "$app/paths";
import type { EntityRef, SearchHit } from "$lib/api/bindings";

/** The client card's tab showing each kind of record. */
const CLIENT_TAB: Partial<Record<SearchHit["kind"], string>> = {
  contact: "contacts",
  interaction: "history",
  note: "notes",
  field: "fields",
  file: "files",
};

/** Where a record opens. Events have no card: the calendar opens. */
function cardHref(ref: EntityRef): string {
  switch (ref.type) {
    case "client":
      return resolve("/clients/[id]", { id: ref.id });
    case "project":
      return resolve("/projects/[id]", { id: ref.id });
    case "task":
      return resolve("/tasks/[id]", { id: ref.id });
    case "secret":
      return resolve("/vault/[id]", { id: ref.id });
    case "invoice":
      return `${resolve("/finance")}?invoice=${encodeURIComponent(ref.id)}`;
    case "event":
      return resolve("/calendar");
    case "expense":
      return resolve("/finance");
    // Contacts and interactions open with their client, which a hit of
    // something attached to them does not know.
    case "contact":
    case "interaction":
      return resolve("/clients");
  }
}

/** Where a search hit opens: its card, or its owner's (on the right tab). */
export function hitHref(hit: SearchHit): string {
  const owner = hit.owner ?? null;
  if (owner) {
    const tab = CLIENT_TAB[hit.kind];
    if (owner.type === "client" && tab) return `${cardHref(owner)}?tab=${tab}`;
    return cardHref(owner);
  }
  switch (hit.kind) {
    case "client":
    case "project":
    case "task":
    case "secret":
    case "invoice":
    case "event":
      return cardHref({ type: hit.kind, id: hit.id });
    default:
      return resolve("/");
  }
}
