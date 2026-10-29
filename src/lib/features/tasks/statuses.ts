import type { TaskStatus } from "$lib/api/bindings";
import { m } from "$lib/paraglide/messages";

const DEFAULT_NAMES: Record<string, () => string> = {
  todo: m.status_todo,
  in_progress: m.status_in_progress,
  review: m.status_review,
  done: m.status_done,
};

/** User-given name, or the localized name of an untouched default column. */
export function statusName(status: Pick<TaskStatus, "name" | "nameKey">): string {
  if (status.name) return status.name;
  const key = status.nameKey ?? "";
  return DEFAULT_NAMES[key]?.() ?? key;
}
