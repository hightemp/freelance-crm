import { listen } from "@tauri-apps/api/event";
import { commands, type VaultStatus } from "$lib/api/bindings";
import { call } from "$lib/api/client";

/** Name of the backend event sent when the vault locks by itself. */
export const VAULT_LOCKED_EVENT = "vault://locked";

/** Vault state shared by the vault screen, the indicator and secrets. */
export const vault = $state<{ status: VaultStatus | null }>({ status: null });

export async function refreshVault(): Promise<void> {
  vault.status = await call(commands.vaultStatus());
}

export async function lockVault(): Promise<void> {
  await commands.vaultLock();
  vault.status = "locked";
}

/** Follows auto-locks from the backend; returns an unsubscribe function. */
export function watchVault(): () => void {
  let unlisten: (() => void) | undefined;
  let stopped = false;
  // Failing to (un)subscribe must not break the UI: without the event the
  // state is still refreshed on the next vault action.
  // Unlisten functions may throw or return a rejected promise when there is
  // no event system (tests) or the listener is gone already.
  const safely = (fn: () => void) => {
    void Promise.resolve()
      .then(fn)
      .catch(() => {});
  };
  listen(VAULT_LOCKED_EVENT, () => {
    vault.status = "locked";
  }).then(
    (fn) => {
      if (stopped) safely(fn);
      else unlisten = fn;
    },
    () => {},
  );
  return () => {
    stopped = true;
    if (unlisten) safely(unlisten);
  };
}

/** Forgets the known state (tests). */
export function resetVault(): void {
  vault.status = null;
}
