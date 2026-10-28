export type ToastKind = "info" | "success" | "error";

export interface Toast {
  id: number;
  message: string;
  kind: ToastKind;
  /** Milliseconds before auto-dismiss; 0 keeps the toast until dismissed. */
  duration: number;
}

const MAX_VISIBLE = 3;

let all = $state.raw<Toast[]>([]);
// Plain Map on purpose: timers are bookkeeping, not rendered state.
// eslint-disable-next-line svelte/prefer-svelte-reactivity
const timers = new Map<number, ReturnType<typeof setTimeout>>();
let nextId = 1;

export function visibleToasts(): Toast[] {
  return all.slice(0, MAX_VISIBLE);
}

// Timers start only when a toast becomes visible, so queued toasts get
// their full duration.
function schedule() {
  for (const t of visibleToasts()) {
    if (t.duration > 0 && !timers.has(t.id)) {
      timers.set(
        t.id,
        setTimeout(() => toast.dismiss(t.id), t.duration),
      );
    }
  }
}

export const toast = {
  show({
    message,
    kind = "info",
    duration = 4000,
  }: {
    message: string;
    kind?: ToastKind;
    duration?: number;
  }): number {
    const id = nextId++;
    all = [...all, { id, message, kind, duration }];
    schedule();
    return id;
  },

  dismiss(id: number): void {
    clearTimeout(timers.get(id));
    timers.delete(id);
    all = all.filter((t) => t.id !== id);
    schedule();
  },
};

export function resetToasts(): void {
  for (const timer of timers.values()) clearTimeout(timer);
  timers.clear();
  all = [];
}
