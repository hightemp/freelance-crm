import type { DuePreset, Priority, TaskSort } from "$lib/api/bindings";
import { m } from "$lib/paraglide/messages";

/** Most urgent first. */
export const PRIORITIES: readonly Priority[] = ["urgent", "high", "normal", "low"];

export function priorityLabel(priority: Priority): string {
  switch (priority) {
    case "low":
      return m.priority_low();
    case "normal":
      return m.priority_normal();
    case "high":
      return m.priority_high();
    case "urgent":
      return m.priority_urgent();
  }
}

export const DUE_PRESETS: readonly DuePreset[] = ["overdue", "today", "week"];

export function dueLabel(due: DuePreset): string {
  switch (due) {
    case "overdue":
      return m.tasks_due_overdue();
    case "today":
      return m.tasks_due_today();
    case "week":
      return m.tasks_due_week();
  }
}

export const SORTS: readonly TaskSort[] = ["board", "priority", "due", "created"];

export function sortLabel(sort: TaskSort): string {
  switch (sort) {
    case "board":
      return m.tasks_sort_board();
    case "priority":
      return m.tasks_sort_priority();
    case "due":
      return m.tasks_sort_due();
    case "created":
      return m.tasks_sort_created();
  }
}
