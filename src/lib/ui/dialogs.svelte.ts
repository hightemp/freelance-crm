// Promise-based replacements for window.confirm / window.prompt (FR-UI-2).
// Requests are queued and rendered one at a time by <DialogHost>.

export interface ConfirmOptions {
  title: string;
  message?: string;
  confirmLabel?: string;
  cancelLabel?: string;
  /** Destructive action: red button, focus starts on Cancel. */
  danger?: boolean;
}

export interface PromptOptions {
  title: string;
  label: string;
  message?: string;
  initial?: string;
  confirmLabel?: string;
  cancelLabel?: string;
}

export type DialogRequest =
  | { kind: "confirm"; options: ConfirmOptions; resolve: (ok: boolean) => void }
  | { kind: "prompt"; options: PromptOptions; resolve: (value: string | null) => void };

let queue = $state.raw<DialogRequest[]>([]);

export function currentDialog(): DialogRequest | undefined {
  return queue[0];
}

export function confirm(options: ConfirmOptions): Promise<boolean> {
  return new Promise((resolve) => {
    queue = [...queue, { kind: "confirm", options, resolve }];
  });
}

export function prompt(options: PromptOptions): Promise<string | null> {
  return new Promise((resolve) => {
    queue = [...queue, { kind: "prompt", options, resolve }];
  });
}

/** Resolves the current request and shows the next one. */
export function settleDialog(value: boolean | string | null): void {
  const [current, ...rest] = queue;
  if (!current) return;
  queue = rest;
  if (current.kind === "confirm") current.resolve(value === true);
  else current.resolve(typeof value === "string" ? value : null);
}

/** Cancels all pending requests (tests, app teardown). */
export function resetDialogs(): void {
  const pending = queue;
  queue = [];
  for (const request of pending) {
    if (request.kind === "confirm") request.resolve(false);
    else request.resolve(null);
  }
}
