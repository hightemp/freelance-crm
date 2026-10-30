import { cleanup, render, screen, waitFor, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import type { TaxRegime, TaxRegimeInput } from "$lib/api/bindings";
import { appSettings, DEFAULT_SETTINGS } from "$lib/app/settings.svelte";
import DialogHost from "$lib/ui/DialogHost.svelte";
import { resetDialogs } from "$lib/ui/dialogs.svelte";
import { expectNoA11yViolations } from "../../../test/a11y";
import TaxSettings from "./TaxSettings.svelte";

let regimes: TaxRegime[] = [];
let calls: { cmd: string; args: Record<string, unknown> }[] = [];

const NPD: TaxRegimeInput = {
  name: "НПД",
  preset: "npd",
  validFrom: "2026-01-01",
  rules: [
    { rateBp: 400, clientKind: "person", currency: null },
    { rateBp: 600, clientKind: "company", currency: null },
  ],
};

beforeEach(() => {
  calls = [];
  regimes = [];
  appSettings.value = { ...DEFAULT_SETTINGS };
  mockIPC((cmd, args) => {
    const a = (args ?? {}) as Record<string, unknown>;
    calls.push({ cmd, args: a });
    switch (cmd) {
      case "list_tax_regimes":
        return regimes;
      case "tax_presets":
        return [{ key: "npd", labels: { en: "Professional income tax (NPD)", ru: "НПД" } }];
      case "tax_preset":
        return a.key === "npd"
          ? { ...NPD, validFrom: a.validFrom }
          : {
              name: "",
              preset: "custom",
              validFrom: a.validFrom,
              rules: [{ rateBp: 0, clientKind: null, currency: null }],
            };
      case "save_tax_regime": {
        const saved = {
          id: (a.id as string) ?? "r1",
          createdAt: 1,
          ...(a.input as TaxRegimeInput),
        };
        regimes = [...regimes.filter((r) => r.id !== saved.id), saved];
        return saved;
      }
      case "delete_tax_regime":
        regimes = regimes.filter((r) => r.id !== a.id);
        return null;
      case "save_settings":
        return a.settings;
    }
  });
});

afterEach(() => {
  cleanup();
  clearMocks();
  resetDialogs();
});

const named = (cmd: string) => calls.filter((c) => c.cmd === cmd).map((c) => c.args);

describe("TaxSettings", () => {
  it("switches taxes on and says it is an estimate", async () => {
    render(TaxSettings, { today: "2026-03-18" });
    expect(screen.getByText(/an estimate, not accounting/)).toBeInTheDocument();
    await userEvent.click(screen.getByRole("switch", { name: "Account for taxes" }));
    await waitFor(() =>
      expect(named("save_settings")[0]?.settings).toMatchObject({ taxesEnabled: true }),
    );
  });

  it("adds a regime from a country preset", async () => {
    render(TaxSettings, { today: "2026-03-18" });
    await userEvent.click(await screen.findByRole("button", { name: "Add regime" }));
    const dialog = await screen.findByRole("dialog", { name: "Tax regime" });
    await userEvent.click(within(dialog).getByRole("combobox", { name: "Preset" }));
    await userEvent.click(
      await screen.findByRole("option", { name: "Professional income tax (NPD)" }),
    );
    const rates = await within(dialog).findAllByLabelText(/^Rate, %/);
    expect(rates.map((r) => (r as HTMLInputElement).value)).toEqual(["4", "6"]);
    await userEvent.click(within(dialog).getByRole("button", { name: "Save" }));
    await waitFor(() => expect(named("save_tax_regime")).toHaveLength(1));
    expect(named("save_tax_regime")[0]).toEqual({
      id: null,
      input: { ...NPD, validFrom: "2026-03-18" },
    });
    expect(await screen.findByText("НПД")).toBeInTheDocument();
  });

  it("edits rules of a custom regime", async () => {
    render(TaxSettings, { today: "2026-03-18" });
    await userEvent.click(await screen.findByRole("button", { name: "Add regime" }));
    const dialog = await screen.findByRole("dialog", { name: "Tax regime" });
    await userEvent.type(within(dialog).getByLabelText(/^Name/), "Мой режим");
    const rate = within(dialog).getByLabelText(/^Rate, %/);
    await userEvent.clear(rate);
    await userEvent.type(rate, "12.5");
    await userEvent.click(within(dialog).getByRole("button", { name: "Add rule" }));
    const rows = within(dialog).getAllByRole("group", { name: /^Rule \d/ });
    expect(rows).toHaveLength(2);
    await userEvent.click(within(rows[1]).getByRole("combobox", { name: "Clients" }));
    await userEvent.click(await screen.findByRole("option", { name: "Companies" }));
    await userEvent.click(within(dialog).getByRole("button", { name: "Remove rule 1" }));
    await userEvent.click(within(dialog).getByRole("button", { name: "Save" }));
    await waitFor(() => expect(named("save_tax_regime")).toHaveLength(1));
    expect(named("save_tax_regime")[0].input).toEqual({
      name: "Мой режим",
      preset: "custom",
      validFrom: "2026-03-18",
      rules: [{ rateBp: 0, clientKind: "company", currency: null }],
    });
  });

  it("lists regimes by date and deletes them", async () => {
    regimes = [{ id: "r1", createdAt: 1, ...NPD }];
    render(DialogHost);
    render(TaxSettings, { today: "2026-03-18" });
    expect(await screen.findByText("НПД")).toBeInTheDocument();
    expect(screen.getByText("from Jan 1, 2026")).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Delete regime НПД" }));
    await userEvent.click(
      within(await screen.findByRole("alertdialog")).getByRole("button", { name: "Delete" }),
    );
    await waitFor(() => expect(screen.queryByText("НПД")).not.toBeInTheDocument());
  });

  it("has no a11y violations", async () => {
    regimes = [{ id: "r1", createdAt: 1, ...NPD }];
    const { container } = render(TaxSettings, { today: "2026-03-18" });
    await screen.findByText("НПД");
    await expectNoA11yViolations(container);
  });
});
