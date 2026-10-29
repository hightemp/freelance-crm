import { act, cleanup, render, screen, waitFor, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { emit } from "@tauri-apps/api/event";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { TimeEntry_Serialize as TimeEntry } from "$lib/api/bindings";
import { expectNoA11yViolations } from "../../../test/a11y";
import { resetTimer } from "./timer.svelte";
import TimerWidget from "./TimerWidget.svelte";

const NOW = Date.parse("2026-03-10T09:00:00Z");

function entry(patch: Partial<TimeEntry> = {}): TimeEntry {
  return {
    id: "t1",
    taskId: null,
    taskTitle: null,
    projectId: "p1",
    projectName: "Сайт",
    clientId: "c1",
    clientName: "Ромашка",
    description: "Вёрстка",
    startedAt: NOW,
    endedAt: null,
    billable: true,
    invoiceId: null,
    createdAt: NOW,
    updatedAt: NOW,
    ...patch,
  };
}

let running: TimeEntry | null = null;
let calls: { cmd: string; args: Record<string, unknown> }[] = [];

beforeEach(() => {
  running = null;
  calls = [];
  resetTimer();
  mockIPC(
    (cmd, args) => {
      const a = (args ?? {}) as Record<string, unknown>;
      calls.push({ cmd, args: a });
      switch (cmd) {
        case "timer_running":
          return running;
        case "timer_start":
          running = entry({ description: a.description as string, startedAt: Date.now() });
          return running;
        case "timer_stop": {
          const stopped = { ...running!, endedAt: Date.now() };
          running = null;
          return stopped;
        }
        case "list_projects":
          return [{ id: "p1", name: "Сайт", clientId: "c1", clientName: "Ромашка" }];
        case "list_tasks":
          return [{ id: "k1", title: "Макет", projectId: "p1" }];
      }
    },
    { shouldMockEvents: true },
  );
});

afterEach(() => {
  cleanup();
  clearMocks();
  vi.useRealTimers();
});

describe("TimerWidget", () => {
  it("starts a timer for a project and stops it", async () => {
    render(TimerWidget);
    await userEvent.click(await screen.findByRole("button", { name: "Start timer" }));
    const panel = await screen.findByRole("dialog", { name: "Start timer" });
    await userEvent.type(within(panel).getByLabelText("What are you working on?"), "Вёрстка");
    await userEvent.click(within(panel).getByRole("combobox", { name: "Project" }));
    await userEvent.click(await screen.findByRole("option", { name: "Сайт · Ромашка" }));
    await userEvent.click(within(panel).getByRole("combobox", { name: "Task" }));
    await userEvent.click(await screen.findByRole("option", { name: "Макет" }));
    await userEvent.click(within(panel).getByRole("button", { name: "Start" }));
    await waitFor(() => expect(calls.some((c) => c.cmd === "timer_start")).toBe(true));
    expect(calls.find((c) => c.cmd === "timer_start")!.args).toEqual({
      target: { taskId: "k1", projectId: "p1", clientId: null },
      description: "Вёрстка",
      billable: true,
    });
    expect(await screen.findByRole("timer")).toHaveTextContent("0:00:00");
    await userEvent.click(screen.getByRole("button", { name: "Stop timer" }));
    expect(await screen.findByRole("button", { name: "Start timer" })).toBeInTheDocument();
  });

  it("ticks every second", async () => {
    vi.useFakeTimers({ toFake: ["Date", "setInterval", "clearInterval"] });
    vi.setSystemTime(NOW + 65_000);
    running = entry();
    render(TimerWidget);
    const clock = await screen.findByRole("timer");
    expect(clock).toHaveTextContent("0:01:05");
    expect(screen.getByText("Вёрстка · Сайт")).toBeInTheDocument();
    await act(() => vi.advanceTimersByTime(2_000));
    expect(clock).toHaveTextContent("0:01:07");
    vi.setSystemTime(NOW + 3 * 3600_000 + 5_000);
    await act(() => vi.advanceTimersByTime(1_000));
    expect(clock).toHaveTextContent("3:00:06");
  });

  it("follows changes from the tray", async () => {
    render(TimerWidget);
    await screen.findByRole("button", { name: "Start timer" });
    running = entry();
    await emit("timer://changed");
    expect(await screen.findByRole("timer")).toBeInTheDocument();
    running = null;
    await emit("timer://changed");
    expect(await screen.findByRole("button", { name: "Start timer" })).toBeInTheDocument();
  });

  it("has no a11y violations", async () => {
    running = entry();
    const { container } = render(TimerWidget);
    await screen.findByRole("timer");
    await expectNoA11yViolations(container);
  });
});
