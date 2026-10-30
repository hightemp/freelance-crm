import { render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Settings_Serialize as Settings } from "$lib/api/bindings";
import { setAppLocale } from "$lib/i18n/locale.svelte";
import { expectNoA11yViolations } from "../../test/a11y";
import SettingsForm from "./SettingsForm.svelte";
import { DEFAULT_SETTINGS, appSettings } from "./settings.svelte";
import { setTheme } from "./theme.svelte";
import { setZoom } from "./zoom.svelte";

let saved: Settings[] = [];
let autostart = false;

beforeEach(() => {
  saved = [];
  autostart = false;
  mockIPC((cmd, args) => {
    if (cmd === "autostart_enabled") return autostart;
    if (cmd === "set_autostart") {
      autostart = (args as { enabled: boolean }).enabled;
      return null;
    }
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
  it("switches closing to the tray and launching at login", async () => {
    render(SettingsForm);
    const tray = screen.getByRole("switch", { name: "Keep running in the tray when closed" });
    expect(tray).toBeChecked();
    await userEvent.click(tray);
    await vi.waitFor(() => expect(saved.at(-1)?.closeToTray).toBe(false));

    const login = screen.getByRole("switch", { name: "Start with the system" });
    await vi.waitFor(() => expect(login).not.toBeChecked());
    await userEvent.click(login);
    await vi.waitFor(() => expect(autostart).toBe(true));
    expect(login).toBeChecked();
  });

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

  it("saves the vault auto-lock timeout", async () => {
    render(SettingsForm);
    expect(
      screen.getByRole("combobox", { name: "Lock the vault after inactivity" }),
    ).toHaveTextContent("15 min");
    await choose("Lock the vault after inactivity", "Never");
    expect(saved.at(-1)?.vaultAutolockMinutes).toBe(0);
    await choose("Lock the vault after inactivity", "5 min");
    expect(saved.at(-1)?.vaultAutolockMinutes).toBe(5);
  });

  it("has no a11y violations", async () => {
    const { container } = render(SettingsForm);
    await expectNoA11yViolations(container);
  });
});
