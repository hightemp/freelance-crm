import { cleanup, render, screen, within } from "@testing-library/svelte";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import type { HomeSummary, Task_Serialize as Task } from "$lib/api/bindings";
import { timer } from "../time/timer.svelte";
import { expectNoA11yViolations } from "../../../test/a11y";
import HomePage from "./HomePage.svelte";

const task = (id: string, title: string, dueDate: string, dueTime: string | null = null) =>
  ({ id, title, dueDate, dueTime, clientName: "Ромашка", projectName: null }) as Task;

const FULL = {
  overdueTasks: [task("t1", "Сдать макет", "2026-03-09")],
  todayTasks: [task("t2", "Созвон", "2026-03-10", "18:00")],
  events: [
    {
      kind: "event",
      id: "e1",
      title: "Встреча",
      allDay: false,
      start: "2026-03-11T10:00",
      end: "2026-03-11T11:00",
      startAt: Date.parse("2026-03-11T10:00:00Z"),
      endAt: Date.parse("2026-03-11T11:00:00Z"),
      clientName: null,
      projectName: null,
      completed: false,
      occurrence: null,
    },
  ],
  openInvoices: [
    {
      id: "i1",
      number: "INV-2026-001",
      clientId: "c1",
      clientName: "Ромашка",
      dueDate: "2026-03-01",
      due: { amountMinor: 150000, currency: "RUB" },
      overdue: true,
    },
  ],
  recentClients: [{ id: "c1", displayName: "Ромашка", activeAt: 1 }],
} as unknown as HomeSummary;

const EMPTY = {
  overdueTasks: [],
  todayTasks: [],
  events: [],
  openInvoices: [],
  recentClients: [],
} as unknown as HomeSummary;

let summary: HomeSummary;

beforeEach(() => {
  summary = FULL;
  timer.running = null;
  timer.loaded = true;
  mockIPC((cmd) => {
    if (cmd === "home_summary") return summary;
  });
});

afterEach(() => {
  cleanup();
  clearMocks();
});

const widget = (name: string) => screen.getByRole("region", { name });

describe("HomePage", () => {
  it("shows each widget's data with links to the records", async () => {
    render(HomePage);
    const overdue = await screen.findByRole("region", { name: "Overdue" });
    expect(within(overdue).getByRole("link", { name: /Сдать макет/ })).toHaveAttribute(
      "href",
      "/tasks/t1",
    );
    expect(within(widget("Today")).getByRole("link", { name: /Созвон/ })).toHaveTextContent(
      "6:00 PM",
    );
    expect(
      within(widget("Upcoming events")).getByRole("link", { name: /Встреча/ }),
    ).toHaveAttribute("href", "/calendar");
    const invoice = within(widget("Unpaid invoices")).getByRole("link", { name: /INV-2026-001/ });
    expect(invoice).toHaveAttribute("href", "/finance?invoice=i1");
    expect(invoice).toHaveTextContent("RUB 1,500.00");
    expect(invoice).toHaveTextContent("Overdue");
    expect(within(widget("Recent clients")).getByRole("link", { name: "Ромашка" })).toHaveAttribute(
      "href",
      "/clients/c1",
    );
  });

  it("shows the running timer", async () => {
    timer.running = {
      id: "r1",
      description: "Вёрстка",
      startedAt: Date.now() - 65_000,
      taskTitle: null,
      projectName: "Сайт",
      clientName: null,
    } as never;
    render(HomePage);
    const widgetTimer = await screen.findByRole("region", { name: "Timer" });
    expect(within(widgetTimer).getByRole("timer")).toHaveTextContent("0:01:05");
    expect(widgetTimer).toHaveTextContent("Вёрстка");
  });

  it("has empty states", async () => {
    summary = EMPTY;
    render(HomePage);
    expect(await screen.findByText("Nothing overdue")).toBeInTheDocument();
    expect(screen.getByText("Nothing due today")).toBeInTheDocument();
    expect(screen.getByText("No events this week")).toBeInTheDocument();
    expect(screen.getByText("All invoices are paid")).toBeInTheDocument();
    expect(screen.getByText("No clients yet")).toBeInTheDocument();
    expect(
      within(widget("Timer")).getByRole("button", { name: "Start timer" }),
    ).toBeInTheDocument();
  });

  it("has no a11y violations", async () => {
    render(HomePage);
    await screen.findByRole("region", { name: "Overdue" });
    await expectNoA11yViolations(document.body);
  });
});
