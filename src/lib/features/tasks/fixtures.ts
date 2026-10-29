// Test data shared by task component tests.
import type { BoardColumn, Task } from "$lib/api/bindings";

export const COLUMNS: BoardColumn[] = [
  ["s-todo", "todo", "todo"],
  ["s-doing", "in_progress", "in_progress"],
  ["s-review", "review", "in_progress"],
  ["s-done", "done", "done"],
].map(([id, nameKey, category], position) => ({
  id,
  name: null,
  nameKey,
  category: category as BoardColumn["category"],
  color: "#64748b",
  position,
  wipLimit: null,
  taskCount: 0,
  overLimit: false,
}));

export function task(id: string, title: string, patch: Partial<Task> = {}): Task {
  return {
    id,
    title,
    descriptionMd: "",
    statusId: "s-todo",
    priority: "normal",
    projectId: null,
    projectName: null,
    clientId: null,
    clientName: null,
    dueDate: null,
    dueTime: null,
    estimateMinutes: null,
    boardPosition: "V",
    tagIds: [],
    checklistDone: 0,
    checklistTotal: 0,
    completedAt: null,
    createdAt: 0,
    updatedAt: 0,
    ...patch,
  };
}
