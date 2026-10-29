// Test data for calendar components (shapes of crm_core types).
import type {
  CalendarItem_Serialize as CalendarItem,
  Event_Serialize as Event,
} from "$lib/api/bindings";

export function event(id: string, title: string, patch: Partial<Event> = {}): Event {
  return {
    id,
    title,
    descriptionMd: "",
    location: null,
    allDay: false,
    start: "2026-03-18T09:30",
    end: "2026-03-18T10:30",
    tzid: "Europe/Moscow",
    startAt: Date.parse("2026-03-18T06:30:00Z"),
    endAt: Date.parse("2026-03-18T07:30:00Z"),
    clientId: null,
    clientName: null,
    projectId: null,
    projectName: null,
    taskId: null,
    contactId: null,
    rrule: null,
    exdates: [],
    seriesId: null,
    recurrenceId: null,
    uid: `${id}@freelance-crm`,
    createdAt: 1,
    updatedAt: 1,
    ...patch,
  };
}

/** A calendar item for `ev` as one of its occurrences. */
export function eventItem(ev: Event, patch: Partial<CalendarItem> = {}): CalendarItem {
  return {
    kind: "event",
    id: ev.id,
    title: ev.title,
    allDay: ev.allDay,
    start: ev.start,
    end: ev.end,
    startAt: ev.startAt,
    endAt: ev.endAt,
    clientName: ev.clientName,
    projectName: ev.projectName,
    completed: false,
    occurrence: {
      event: ev,
      start: ev.start,
      end: ev.end,
      startAt: ev.startAt,
      endAt: ev.endAt,
      recurrenceId: ev.rrule ? ev.start : ev.recurrenceId,
    },
    ...patch,
  };
}

export function deadline(
  kind: "task_due" | "project_deadline",
  id: string,
  title: string,
  date: string,
  patch: Partial<CalendarItem> = {},
): CalendarItem {
  return {
    kind,
    id,
    title,
    allDay: true,
    start: date,
    end: date,
    startAt: Date.parse(`${date}T00:00:00Z`),
    endAt: Date.parse(`${date}T00:00:00Z`) + 86_400_000,
    clientName: null,
    projectName: null,
    completed: false,
    occurrence: null,
    ...patch,
  };
}
