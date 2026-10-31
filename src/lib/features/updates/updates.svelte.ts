import {
  commands,
  type AvailableUpdate_Serialize as AvailableUpdate,
  type Settings_Serialize as Settings,
} from "$lib/api/bindings";
import { call } from "$lib/api/client";

export const updates = $state({
  /** A newer version found by the last check. */
  available: null as AvailableUpdate | null,
  dialogOpen: false,
});

export async function checkForUpdate(): Promise<AvailableUpdate | null> {
  updates.available = await call(commands.checkForUpdate());
  return updates.available;
}

/** Looks for a new version unless the user turned it off; offline is fine. */
export async function checkAtStartup(settings: Settings): Promise<void> {
  if (!settings.checkUpdates) return;
  await checkForUpdate().catch(() => {});
}

/** Downloads and installs the update; the app restarts when it succeeds. */
export async function installUpdate(): Promise<void> {
  await call(commands.installUpdate());
}

export function resetUpdates(): void {
  updates.available = null;
  updates.dialogOpen = false;
}
