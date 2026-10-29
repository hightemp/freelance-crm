import { cleanup, render, screen, waitFor, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { emit } from "@tauri-apps/api/event";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import type { Candidate } from "$lib/api/bindings";
import { expectNoA11yViolations } from "../../../test/a11y";
import MissedReminders from "./MissedReminders.svelte";

const missed = (title: string, startAt: string, allDay = false): Candidate => ({
  reminderId: `r-${title}`,
  owner: { type: "event", id: `e-${title}` },
  occurrence: "",
  title,
  fireAt: Date.parse(startAt) - 900_000,
  startAt: Date.parse(startAt),
  allDay,
});

let list: Candidate[] = [];
let seen = 0;

beforeEach(() => {
  list = [];
  seen = 0;
  mockIPC(
    (cmd) => {
      switch (cmd) {
        case "missed_reminders":
          return list;
        case "mark_missed_reminders_seen":
          seen++;
          list = [];
          return null;
      }
    },
    { shouldMockEvents: true },
  );
});

afterEach(() => {
  cleanup();
  clearMocks();
});

describe("MissedReminders", () => {
  it("lists reminders missed while the app was closed", async () => {
    list = [
      missed("Созвон", "2026-03-18T10:00:00Z"),
      missed("Отчёт", "2026-03-19T00:00:00Z", true),
    ];
    render(MissedReminders);
    const dialog = await screen.findByRole("dialog", { name: "Missed reminders" });
    const items = within(dialog).getAllByRole("listitem");
    expect(items[0]).toHaveTextContent("Созвон");
    expect(items[0]).toHaveTextContent("Mar 18, 2026, 10:00 AM");
    expect(items[1]).toHaveTextContent("Отчёт");
    expect(items[1]).toHaveTextContent("Mar 19, 2026");
    await userEvent.click(within(dialog).getByRole("button", { name: "Mark as seen" }));
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
    expect(seen).toBe(1);
  });

  it("stays hidden without missed reminders and appears when told", async () => {
    render(MissedReminders);
    await new Promise((r) => setTimeout(r, 20));
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    list = [missed("Созвон", "2026-03-18T10:00:00Z")];
    await emit("reminders://missed");
    expect(await screen.findByRole("dialog", { name: "Missed reminders" })).toBeInTheDocument();
  });

  it("can be closed for now without marking", async () => {
    list = [missed("Созвон", "2026-03-18T10:00:00Z")];
    render(MissedReminders);
    const dialog = await screen.findByRole("dialog", { name: "Missed reminders" });
    await userEvent.click(within(dialog).getByRole("button", { name: "Close" }));
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
    expect(seen).toBe(0);
  });

  it("has no a11y violations", async () => {
    list = [missed("Созвон", "2026-03-18T10:00:00Z")];
    render(MissedReminders);
    await expectNoA11yViolations(await screen.findByRole("dialog", { name: "Missed reminders" }));
  });
});
