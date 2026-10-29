import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { Settings_Serialize as Settings } from "$lib/api/bindings";
import { i18n, setAppLocale } from "$lib/i18n/locale.svelte";
import { uiPrefs } from "$lib/ui/prefs.svelte";
import { resetToasts, visibleToasts } from "$lib/ui/toast.svelte";
import { DEFAULT_SETTINGS, appSettings, loadSettings, updateSettings } from "./settings.svelte";
import { setTheme } from "./theme.svelte";
import { setZoom } from "./zoom.svelte";

const stored: Settings = {
  locale: "ru",
  theme: "dark",
  defaultCurrency: "RUB",
  weekStart: 0,
  zoomPercent: 125,
  vaultAutolockMinutes: 30,
  timerStopPrevious: true,
  timeRoundingMinutes: 0,
};

afterEach(() => {
  clearMocks();
  vi.restoreAllMocks();
  resetToasts();
  setAppLocale("en");
  setTheme("system");
  setZoom(1);
  appSettings.value = { ...DEFAULT_SETTINGS };
});

describe("loadSettings", () => {
  it("loads settings from the backend and applies them", async () => {
    mockIPC((cmd) => (cmd === "get_settings" ? stored : undefined));
    await loadSettings();
    expect(appSettings.value).toEqual(stored);
    expect(i18n.locale).toBe("ru");
    expect(document.documentElement.dataset.theme).toBe("dark");
    expect(document.documentElement.style.getPropertyValue("zoom")).toBe("1.25");
    expect(uiPrefs.weekStart).toBe(0);
  });

  it("uses the OS language when locale is not set", async () => {
    vi.spyOn(navigator, "languages", "get").mockReturnValue(["ru-RU"]);
    mockIPC(() => ({ ...stored, locale: null }));
    await loadSettings();
    expect(i18n.locale).toBe("ru");
  });

  it("keeps defaults and reports when loading fails", async () => {
    mockIPC(() => {
      throw { code: "internal.db", params: {} };
    });
    await loadSettings();
    expect(appSettings.value).toEqual(DEFAULT_SETTINGS);
    expect(visibleToasts()[0]).toMatchObject({ kind: "error" });
  });
});

describe("updateSettings", () => {
  it("applies immediately and saves the merged settings", async () => {
    const saved: unknown[] = [];
    mockIPC((cmd, args) => {
      if (cmd === "save_settings") {
        saved.push((args as { settings: Settings }).settings);
        return (args as { settings: Settings }).settings;
      }
    });
    const pending = updateSettings({ theme: "dark" });
    expect(document.documentElement.dataset.theme).toBe("dark");
    await pending;
    expect(saved).toEqual([{ ...DEFAULT_SETTINGS, theme: "dark" }]);
    expect(appSettings.value.theme).toBe("dark");
  });

  it("reverts and shows a translated error when saving fails", async () => {
    mockIPC(() => {
      throw { code: "validation.range", params: { field: "weekStart" } };
    });
    await updateSettings({ weekStart: 9 });
    expect(appSettings.value.weekStart).toBe(DEFAULT_SETTINGS.weekStart);
    expect(uiPrefs.weekStart).toBe(DEFAULT_SETTINGS.weekStart);
    expect(visibleToasts()[0]).toMatchObject({ kind: "error", message: "Out of range" });
  });
});
