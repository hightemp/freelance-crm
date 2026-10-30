// The running timer, shared by the header widget and the time pages and
// kept in sync with the tray through the `timer://changed` event.
import { listen } from "@tauri-apps/api/event";
import {
  commands,
  type TimeEntry_Serialize as TimeEntry,
  type TimeTarget,
} from "$lib/api/bindings";
import { call } from "$lib/api/client";

export const TIMER_CHANGED_EVENT = "timer://changed";

export const timer = $state<{ running: TimeEntry | null; loaded: boolean }>({
  running: null,
  loaded: false,
});

/** Whether the header's start panel is open; the command palette opens it. */
export const timerPanel = $state({ open: false });

export function resetTimer(): void {
  timer.running = null;
  timer.loaded = false;
}

export async function refreshTimer(): Promise<void> {
  timer.running = await call(commands.timerRunning());
  timer.loaded = true;
}

export async function startTimer(target: TimeTarget, description: string, billable: boolean) {
  timer.running = await call(commands.timerStart(target, description, billable));
}

export async function stopTimer(): Promise<TimeEntry> {
  const stopped = await call(commands.timerStop());
  timer.running = null;
  return stopped;
}

/** Follows changes made elsewhere (the tray); returns the unsubscriber. */
export function watchTimer(onError: (err: unknown) => void): () => void {
  let unlisten: (() => void) | undefined;
  let stopped = false;
  // Unlistening may fail without an event system (tests): never break.
  const safely = (fn: () => void) =>
    void Promise.resolve()
      .then(fn)
      .catch(() => {});
  listen(TIMER_CHANGED_EVENT, () => refreshTimer().catch(onError)).then(
    (fn) => (stopped ? safely(fn) : (unlisten = fn)),
    () => {},
  );
  return () => {
    stopped = true;
    if (unlisten) safely(unlisten);
  };
}

/** `h:mm:ss` of a duration in milliseconds. */
export function formatElapsed(ms: number): string {
  const total = Math.max(0, Math.floor(ms / 1000));
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${Math.floor(total / 3600)}:${pad(Math.floor(total / 60) % 60)}:${pad(total % 60)}`;
}
