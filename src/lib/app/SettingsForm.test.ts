import { render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import type { Settings_Serialize as Settings } from "$lib/api/bindings";
import { setAppLocale } from "$lib/i18n/locale.svelte";
import { expectNoA11yViolations } from "../../test/a11y";
import SettingsForm from "./SettingsForm.svelte";
import { DEFAULT_SETTINGS, appSettings } from "./settings.svelte";
import { setTheme } from "./theme.svelte";
import { setZoom } from "./zoom.svelte";

let saved: Settings[] = [];

beforeEach(() => {
  saved = [];
  mockIPC((cmd, args) => {
    if (cmd === "save_settings") {
      const settings = (args as { settings: Settings }).settings;
      saved.push(settings);
      return settings;
    }
  });
});

afterEach(() => {
  clearMocks();
  setAppLocale("en");
  setTheme("system");
  setZoom(1);
  appSettings.value = { ...DEFAULT_SETTINGS };
});

async function choose(label: string, option: string) {
  await userEvent.click(screen.getByRole("combobox", { name: label }));
  await userEvent.click(screen.getByRole("option", { name: option }));
}

describe("SettingsForm", () => {
  it("shows current values", () => {
    render(SettingsForm);
    expect(screen.getByRole("combobox", { name: "Language" })).toHaveTextContent("System language");
    expect(screen.getByRole("combobox", { name: "Default currency" })).toHaveTextContent("USD");
    expect(screen.getByRole("combobox", { name: "First day of week" })).toHaveTextContent("Monday");
    expect(screen.getByRole("radio", { name: "System" })).toHaveAttribute("aria-checked", "true");
  });

  it("switches language, saves it and re-renders in Russian", async () => {
    render(SettingsForm);
    await choose("Language", "Русский");
    expect(saved.at(-1)?.locale).toBe("ru");
    expect(await screen.findByRole("combobox", { name: "Язык" })).toBeInTheDocument();
  });

  it("saves theme, currency, week start and zoom", async () => {
    render(SettingsForm);
    await userEvent.click(screen.getByRole("radio", { name: "Dark" }));
    expect(saved.at(-1)?.theme).toBe("dark");
    await choose("Default currency", "RUB — Russian Ruble");
    expect(saved.at(-1)?.defaultCurrency).toBe("RUB");
    await choose("First day of week", "Sunday");
    expect(saved.at(-1)?.weekStart).toBe(0);
    await choose("Interface scale", "125%");
    expect(saved.at(-1)?.zoomPercent).toBe(125);
    expect(document.documentElement.style.getPropertyValue("zoom")).toBe("1.25");
  });

  it("has no a11y violations", async () => {
    const { container } = render(SettingsForm);
    await expectNoA11yViolations(container);
  });
});
