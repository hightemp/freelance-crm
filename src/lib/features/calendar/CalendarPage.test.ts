import { cleanup, render, screen, waitFor, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { CalendarItem_Serialize as CalendarItem, CalendarLayers } from "$lib/api/bindings";
import DialogHost from "$lib/ui/DialogHost.svelte";
import { resetDialogs } from "$lib/ui/dialogs.svelte";
import { expectNoA11yViolations } from "../../../test/a11y";
import CalendarPage from "./CalendarPage.svelte";
import { deadline, event, eventItem } from "./fixtures";

const goto = vi.hoisted(() => vi.fn());
vi.mock("$app/navigation", () => ({ goto }));

const at = (s: string) => Date.parse(s);
const meeting = event("e1", "Встреча", {
  tzid: "UTC",
  start: "2026-03-18T09:30",
  end: "2026-03-18T10:30",
  startAt: at("2026-03-18T09:30:00Z"),
  endAt: at("2026-03-18T10:30:00Z"),
});
const busyDay = Array.from({ length: 5 }, (_, i) =>
  eventItem(
    event(`b${i}`, `Дело ${i + 1}`, {
      tzid: "UTC",
      start: `2026-03-20T1${i}:00`,
      end: `2026-03-20T1${i}:30`,
      startAt: at(`2026-03-20T1${i}:00:00Z`),
      endAt: at(`2026-03-20T1${i}:30:00Z`),
    }),
  ),
);
const ITEMS: CalendarItem[] = [
  deadline("task_due", "t1", "Сдать макет", "2026-03-18"),
  eventItem(meeting),
  ...busyDay,
  deadline("project_deadline", "p1", "Сайт", "2026-03-25"),
];

let calls: { cmd: string; args: Record<string, unknown> }[] = [];

beforeEach(() => {
  calls = [];
  mockIPC((cmd, args) => {
    const a = (args ?? {}) as Record<string, unknown>;
    calls.push({ cmd, args: a });
    switch (cmd) {
      case "calendar_items": {
        const layers = a.layers as CalendarLayers;
        return ITEMS.filter(
          (i) =>
            i.endAt > (a.from as number) &&
            i.startAt < (a.to as number) &&
            ((i.kind === "event" && layers.events) ||
              (i.kind === "task_due" && layers.tasks) ||
              (i.kind === "project_deadline" && layers.projects)),
        );
      }
      case "list_clients":
      case "list_projects":
      case "list_reminders":
        return [];
    }
  });
});

afterEach(() => {
  cleanup();
  clearMocks();
  resetDialogs();
  goto.mockReset();
});

const lastRange = () => {
  const c = calls.findLast((c) => c.cmd === "calendar_items")!;
  return [
    new Date(c.args.from as number).toISOString(),
    new Date(c.args.to as number).toISOString(),
  ];
};

function show() {
  render(DialogHost);
  return render(CalendarPage, { today: "2026-03-18", timeZone: "UTC" });
}

describe("CalendarPage", () => {
  it("shows a month with events and deadlines", async () => {
    show();
    expect(await screen.findByRole("heading", { name: "March 2026" })).toBeInTheDocument();
    expect(await screen.findByRole("button", { name: /Встреча/ })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Сдать макет/ })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Сайт/ })).toBeInTheDocument();
    expect(lastRange()).toEqual(["2026-02-23T00:00:00.000Z", "2026-04-06T00:00:00.000Z"]);
  });

  it("collapses a crowded day into “+N more” that opens the day", async () => {
    show();
    const more = await screen.findByRole("button", { name: "+2 more" });
    expect(screen.queryByRole("button", { name: /Дело 5/ })).not.toBeInTheDocument();
    await userEvent.click(more);
    expect(
      await screen.findByRole("heading", { name: "Friday, March 20, 2026" }),
    ).toBeInTheDocument();
    expect(await screen.findByRole("button", { name: /Дело 5/ })).toBeInTheDocument();
  });

  it("switches views and moves through periods", async () => {
    show();
    await screen.findByRole("button", { name: /Встреча/ });
    await userEvent.click(screen.getByRole("button", { name: "Week" }));
    await waitFor(() =>
      expect(lastRange()).toEqual(["2026-03-16T00:00:00.000Z", "2026-03-23T00:00:00.000Z"]),
    );
    expect(await screen.findByRole("button", { name: /Встреча/ })).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Next" }));
    await waitFor(() => expect(lastRange()[0]).toBe("2026-03-23T00:00:00.000Z"));
    expect(await screen.findByRole("button", { name: /Сайт/ })).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Today" }));
    await waitFor(() => expect(lastRange()[0]).toBe("2026-03-16T00:00:00.000Z"));
    await userEvent.click(screen.getByRole("button", { name: "Agenda" }));
    expect(
      await screen.findByRole("heading", { name: "Wednesday, March 18, 2026" }),
    ).toBeInTheDocument();
    expect(lastRange()).toEqual(["2026-03-18T00:00:00.000Z", "2026-04-17T00:00:00.000Z"]);
  });

  it("hides a layer that is switched off", async () => {
    show();
    await screen.findByRole("button", { name: /Сдать макет/ });
    await userEvent.click(screen.getByRole("button", { name: "Layers" }));
    await userEvent.click(screen.getByRole("checkbox", { name: "Task deadlines" }));
    await waitFor(() =>
      expect(screen.queryByRole("button", { name: /Сдать макет/ })).not.toBeInTheDocument(),
    );
    expect(
      (calls.findLast((c) => c.cmd === "calendar_items")!.args.layers as CalendarLayers).tasks,
    ).toBe(false);
    expect(screen.getByRole("button", { name: /Встреча/ })).toBeInTheDocument();
  });

  it("opens a new event at the clicked time slot", async () => {
    show();
    await userEvent.click(await screen.findByRole("button", { name: "Week" }));
    await userEvent.click(
      await screen.findByRole("button", { name: "New event at Thu, Mar 19, 2:30 PM" }),
    );
    const dialog = await screen.findByRole("dialog", { name: "New event" });
    expect(within(dialog).getByLabelText("Start time")).toHaveValue("2:30 PM");
    expect(within(dialog).getByLabelText("End time")).toHaveValue("3:30 PM");
  });

  it("opens events for editing and deadlines in their cards", async () => {
    show();
    await userEvent.click(await screen.findByRole("button", { name: /Встреча/ }));
    expect(await screen.findByRole("dialog", { name: "Edit event" })).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Cancel" }));
    await userEvent.click(screen.getByRole("button", { name: /Сдать макет/ }));
    expect(goto).toHaveBeenCalledWith("/tasks/t1");
  });

  it("has no a11y violations", async () => {
    const { container } = show();
    await screen.findByRole("button", { name: /Встреча/ });
    await expectNoA11yViolations(container);
  });
});
