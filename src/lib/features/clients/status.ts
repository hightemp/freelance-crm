import type { ClientStatus } from "$lib/api/bindings";
import { m } from "$lib/paraglide/messages";

/** Lifecycle order, also used for sorting. */
export const CLIENT_STATUSES: readonly ClientStatus[] = ["lead", "active", "paused", "former"];

export function statusLabel(status: ClientStatus): string {
  switch (status) {
    case "lead":
      return m.client_status_lead();
    case "active":
      return m.client_status_active();
    case "paused":
      return m.client_status_paused();
    case "former":
      return m.client_status_former();
  }
}
