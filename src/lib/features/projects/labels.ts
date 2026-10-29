import type { BillingType, ProjectStatus } from "$lib/api/bindings";
import { m } from "$lib/paraglide/messages";

/** Lifecycle order, also used for sorting. */
export const PROJECT_STATUSES: readonly ProjectStatus[] = [
  "lead",
  "active",
  "paused",
  "completed",
  "cancelled",
];

export function projectStatusLabel(status: ProjectStatus): string {
  switch (status) {
    case "lead":
      return m.project_status_lead();
    case "active":
      return m.project_status_active();
    case "paused":
      return m.project_status_paused();
    case "completed":
      return m.project_status_completed();
    case "cancelled":
      return m.project_status_cancelled();
  }
}

export function billingLabel(type: BillingType): string {
  return type === "fixed" ? m.project_billing_fixed() : m.project_billing_hourly();
}
