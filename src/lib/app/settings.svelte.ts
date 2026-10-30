import { commands, type Settings_Serialize as Settings } from "$lib/api/bindings";
import { call } from "$lib/api/client";
import { errorMessage } from "$lib/i18n/errors";
import { detectLocale, setAppLocale } from "$lib/i18n/locale.svelte";
import { uiPrefs } from "$lib/ui/prefs.svelte";
import { toast } from "$lib/ui/toast.svelte";
import { setTheme } from "./theme.svelte";
import { setZoom } from "./zoom.svelte";

export const DEFAULT_SETTINGS: Settings = {
  locale: null,
  theme: "system",
  defaultCurrency: "USD",
  weekStart: 1,
  zoomPercent: 100,
  vaultAutolockMinutes: 15,
  timerStopPrevious: true,
  timeRoundingMinutes: 0,
  invoiceNumberTemplate: "INV-{YYYY}-{NNN}",
  invoiceShowSignature: false,
  taxesEnabled: false,
  autoBackupSchedule: "off",
  autoBackupDir: null,
  autoBackupKeep: 7,
  autoBackupKey: "vault",
};

export const appSettings = $state({ value: { ...DEFAULT_SETTINGS } });

function apply(s: Settings) {
  setAppLocale(s.locale ?? detectLocale(navigator.languages));
  setTheme(s.theme);
  setZoom(s.zoomPercent / 100);
  uiPrefs.weekStart = s.weekStart;
}

export async function loadSettings(): Promise<void> {
  try {
    appSettings.value = await call(commands.getSettings());
  } catch (err) {
    toast.show({ kind: "error", message: errorMessage(err) });
  }
  apply(appSettings.value);
}

/** Applies the change right away, saves it, and reverts if saving fails. */
export async function updateSettings(patch: Partial<Settings>): Promise<void> {
  const previous = appSettings.value;
  const next = { ...previous, ...patch };
  appSettings.value = next;
  apply(next);
  try {
    appSettings.value = await call(commands.saveSettings(next));
  } catch (err) {
    appSettings.value = previous;
    apply(previous);
    toast.show({ kind: "error", message: errorMessage(err) });
  }
}
