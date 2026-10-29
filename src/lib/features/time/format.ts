import type { TimeEntry_Serialize as TimeEntry } from "$lib/api/bindings";

/** `h:mm` of seconds. */
export function hoursMinutes(seconds: number): string {
  const minutes = Math.round(seconds / 60);
  return `${Math.floor(minutes / 60)}:${String(minutes % 60).padStart(2, "0")}`;
}

/** "Task · description": what the entry is for. */
export function entryLabel(e: TimeEntry): string {
  return [e.taskTitle ?? e.projectName ?? e.clientName, e.description].filter(Boolean).join(" · ");
}

/** Timestamp of a local `YYYY-MM-DDTHH:MM`. */
export function fromLocal(text: string): number {
  const [date, time] = text.split("T");
  const [y, m, d] = date.split("-").map(Number);
  const [h, min] = time.split(":").map(Number);
  return new Date(y, m - 1, d, h, min).getTime();
}
