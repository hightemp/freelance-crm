import { cleanup, render, screen, waitFor, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import DialogHost from "$lib/ui/DialogHost.svelte";
import { resetDialogs } from "$lib/ui/dialogs.svelte";
import { expectNoA11yViolations } from "../../../test/a11y";
import EventDialog from "./EventDialog.svelte";
import { event } from "./fixtures";

let calls: { cmd: string; args: Record<string, unknown> }[] = [];
let reminders: number[] = [];

beforeEach(() => {
  calls = [];
  reminders = [];
  mockIPC((cmd, args) => {
    const a = (args ?? {}) as Record<string, unknown>;
    calls.push({ cmd, args: a });
    switch (cmd) {
      case "list_clients":
      case "list_projects":
        return [];
      case "list_reminders":
        return reminders.map((m, i) => ({ id: `r${i}`, minutesBefore: m }));
      case "set_reminders":
        return [];
      case "create_event":
        if ((a.input as { end: string }).end < (a.input as { start: string }).start)
          throw { code: "validation.before_start", params: { field: "end" } };
        return event("e9", "x");
      case "update_event":
      case "update_occurrence":
        return event("e9", "x");
      case "delete_event":
      case "delete_occurrence":
        return null;
    }
  });
});

afterEach(() => {
  cleanup();
  clearMocks();
  resetDialogs();
});

const NEW = { start: "2026-03-18T09:30", end: "2026-03-18T10:30", allDay: false };
const label = (name: string) => new RegExp(`^${name}\\s*\\*?$`);
const named = (cmd: string) => calls.filter((c) => c.cmd === cmd).map((c) => c.args);

function open(props: Record<string, unknown> = {}) {
  const onsaved = vi.fn();
  const onclose = vi.fn();
  render(DialogHost);
  render(EventDialog, { initial: NEW, timeZone: "Europe/Moscow", onsaved, onclose, ...props });
  return { onsaved, onclose };
}

describe("EventDialog: new events", () => {
  it("creates an event at the chosen time with reminders", async () => {
    const { onsaved } = open();
    const dialog = await screen.findByRole("dialog", { name: "New event" });
    await userEvent.type(
      within(dialog).getByLabelText(label("Title"), { selector: "input" }),
      "Созвон",
    );
    await userEvent.click(within(dialog).getByRole("combobox", { name: "Add a reminder" }));
    await userEvent.click(screen.getByRole("option", { name: "15 min before" }));
    await userEvent.click(within(dialog).getByRole("combobox", { name: "Add a reminder" }));
    await userEvent.click(screen.getByRole("option", { name: "1 d before" }));
    expect(
      within(dialog).getByRole("button", { name: "Remove reminder: 15 min before" }),
    ).toBeInTheDocument();
    await userEvent.click(within(dialog).getByRole("button", { name: "Save" }));
    await waitFor(() => expect(onsaved).toHaveBeenCalled());
    expect(named("create_event")[0].input).toEqual({
      title: "Созвон",
      descriptionMd: "",
      location: null,
      allDay: false,
      start: "2026-03-18T09:30",
      end: "2026-03-18T10:30",
      tzid: "Europe/Moscow",
      clientId: null,
      projectId: null,
      taskId: null,
      contactId: null,
      rrule: null,
    });
    expect(named("set_reminders")[0]).toEqual({
      owner: { type: "event", id: "e9" },
      minutesBefore: [15, 1440],
    });
  });

  it("makes all-day events from dates", async () => {
    open();
    const dialog = await screen.findByRole("dialog", { name: "New event" });
    await userEvent.type(
      within(dialog).getByLabelText(label("Title"), { selector: "input" }),
      "Отпуск",
    );
    await userEvent.click(within(dialog).getByRole("switch", { name: "All day" }));
    expect(within(dialog).queryByLabelText("Start time")).not.toBeInTheDocument();
    await userEvent.click(within(dialog).getByRole("button", { name: "Save" }));
    await waitFor(() => expect(named("create_event")).toHaveLength(1));
    expect(named("create_event")[0].input).toMatchObject({
      allDay: true,
      start: "2026-03-18",
      end: "2026-03-18",
    });
  });

  it("builds the repeat rule from the presets", async () => {
    open();
    const dialog = await screen.findByRole("dialog", { name: "New event" });
    await userEvent.type(
      within(dialog).getByLabelText(label("Title"), { selector: "input" }),
      "Планёрка",
    );
    await userEvent.click(within(dialog).getByRole("combobox", { name: "Repeat" }));
    await userEvent.click(screen.getByRole("option", { name: /^Every week on/ }));
    await userEvent.click(within(dialog).getByRole("combobox", { name: "Ends" }));
    await userEvent.click(screen.getByRole("option", { name: "After a number of times" }));
    const times = within(dialog).getByLabelText(label("Times"), { selector: "input" });
    await userEvent.clear(times);
    await userEvent.type(times, "5");
    await userEvent.click(within(dialog).getByRole("button", { name: "Save" }));
    await waitFor(() => expect(named("create_event")).toHaveLength(1));
    expect((named("create_event")[0].input as { rrule: string }).rrule).toBe(
      "FREQ=WEEKLY;BYDAY=WE;COUNT=5",
    );
  });

  it("shows backend validation errors at the field", async () => {
    open({ initial: { start: "2026-03-18T09:30", end: "2026-03-18T08:00", allDay: false } });
    const dialog = await screen.findByRole("dialog", { name: "New event" });
    await userEvent.type(within(dialog).getByLabelText(label("Title"), { selector: "input" }), "x");
    await userEvent.click(within(dialog).getByRole("button", { name: "Save" }));
    expect(await within(dialog).findByText("Cannot be before the start")).toBeInTheDocument();
    expect(named("set_reminders")).toHaveLength(0);
  });

  it("asks for a title", async () => {
    open();
    const dialog = await screen.findByRole("dialog", { name: "New event" });
    await userEvent.click(within(dialog).getByRole("button", { name: "Save" }));
    expect(
      within(dialog).getByLabelText(label("Title"), { selector: "input" }),
    ).toHaveAccessibleDescription("This field is required");
    expect(named("create_event")).toHaveLength(0);
  });
});

describe("EventDialog: repeating events", () => {
  const series = event("s1", "Планёрка", {
    start: "2026-03-02T10:00",
    end: "2026-03-02T10:30",
    rrule: "FREQ=WEEKLY;BYDAY=MO",
  });
  // The occurrence of Monday, March 16th.
  const occurrence = {
    recurrenceId: "2026-03-16T10:00",
    start: "2026-03-16T10:00",
    end: "2026-03-16T10:30",
  };

  it("loads the series and its reminders", async () => {
    reminders = [10];
    open({ event: series, occurrence });
    const dialog = await screen.findByRole("dialog", { name: "Edit event" });
    expect(within(dialog).getByLabelText(label("Title"), { selector: "input" })).toHaveValue(
      "Планёрка",
    );
    expect(
      await within(dialog).findByRole("button", { name: "Remove reminder: 10 min before" }),
    ).toBeInTheDocument();
    expect(within(dialog).getByRole("combobox", { name: "Repeat" })).toHaveTextContent(
      /^Every week on/,
    );
  });

  it("asks which occurrences a change applies to", async () => {
    const { onsaved } = open({ event: series, occurrence });
    const dialog = await screen.findByRole("dialog", { name: "Edit event" });
    const title = within(dialog).getByLabelText(label("Title"), { selector: "input" });
    await userEvent.clear(title);
    await userEvent.type(title, "Планёрка (короткая)");
    await userEvent.click(within(dialog).getByRole("button", { name: "Save" }));
    const scope = await screen.findByRole("dialog", { name: "Change a repeating event" });
    await userEvent.click(within(scope).getByRole("radio", { name: "This event" }));
    await userEvent.click(within(scope).getByRole("button", { name: "OK" }));
    await waitFor(() => expect(onsaved).toHaveBeenCalled());
    expect(named("update_occurrence")[0]).toMatchObject({
      id: "s1",
      recurrenceId: "2026-03-16T10:00",
      scope: "this",
      input: { title: "Планёрка (короткая)", start: "2026-03-16T10:00", rrule: null },
    });
  });

  it("applies a change to all from the series start", async () => {
    open({ event: series, occurrence });
    const dialog = await screen.findByRole("dialog", { name: "Edit event" });
    await userEvent.click(within(dialog).getByRole("button", { name: "Save" }));
    const scope = await screen.findByRole("dialog", { name: "Change a repeating event" });
    await userEvent.click(within(scope).getByRole("radio", { name: "All events" }));
    await userEvent.click(within(scope).getByRole("button", { name: "OK" }));
    await waitFor(() => expect(named("update_occurrence")).toHaveLength(1));
    expect(named("update_occurrence")[0]).toMatchObject({
      scope: "all",
      input: { start: "2026-03-02T10:00", end: "2026-03-02T10:30", rrule: "FREQ=WEEKLY;BYDAY=MO" },
    });
  });

  it("deletes this and following occurrences", async () => {
    const { onsaved } = open({ event: series, occurrence });
    const dialog = await screen.findByRole("dialog", { name: "Edit event" });
    await userEvent.click(within(dialog).getByRole("button", { name: "Delete event" }));
    const scope = await screen.findByRole("dialog", { name: "Delete a repeating event" });
    await userEvent.click(within(scope).getByRole("radio", { name: "This and following events" }));
    await userEvent.click(within(scope).getByRole("button", { name: "OK" }));
    await waitFor(() => expect(onsaved).toHaveBeenCalled());
    expect(named("delete_occurrence")).toEqual([
      { id: "s1", recurrenceId: "2026-03-16T10:00", scope: "following" },
    ]);
  });

  it("deletes a single event after confirmation", async () => {
    open({ event: event("e1", "Встреча"), occurrence: null });
    const dialog = await screen.findByRole("dialog", { name: "Edit event" });
    await userEvent.click(within(dialog).getByRole("button", { name: "Delete event" }));
    const confirm = await screen.findByRole("alertdialog", { name: "Delete the event?" });
    await userEvent.click(within(confirm).getByRole("button", { name: "Delete" }));
    await waitFor(() => expect(named("delete_event")).toEqual([{ id: "e1" }]));
  });
});

describe("EventDialog: a11y", () => {
  it("has no violations", async () => {
    open();
    const dialog = await screen.findByRole("dialog", { name: "New event" });
    await expectNoA11yViolations(dialog);
  });
});
