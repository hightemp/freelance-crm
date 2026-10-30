// Accessibility of whole screens (NFR-7): every main route, rendered as
// the user first sees it, has no axe violations. Feature components have
// their own checks with data; this covers what the routes add around them.
import { cleanup, render, screen } from "@testing-library/svelte";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import type { Component } from "svelte";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { resetSchemas } from "$lib/features/requisites/schemas";
import { resetVault } from "$lib/features/vault/vault.svelte";
import { expectNoA11yViolations } from "../test/a11y";
import Calendar from "./calendar/+page.svelte";
import Clients from "./clients/+page.svelte";
import NewClient from "./clients/new/+page.svelte";
import Finance from "./finance/+page.svelte";
import Home from "./+page.svelte";
import Projects from "./projects/+page.svelte";
import Settings from "./settings/+page.svelte";
import Tasks from "./tasks/+page.svelte";
import Time from "./time/+page.svelte";
import Vault from "./vault/+page.svelte";

const pageState = vi.hoisted(() => ({ url: new URL("http://localhost/"), params: {} }));
vi.mock("$app/state", () => ({ page: pageState }));
vi.mock("$app/navigation", () => ({ goto: vi.fn() }));

const EMPTY_TOTAL = { key: "", label: "", seconds: 0, billableSeconds: 0, amounts: [] };

beforeEach(() => {
  resetVault();
  mockIPC(
    (cmd) => {
      switch (cmd) {
        case "vault_status":
          return "uninitialized";
        case "home_summary":
          return {
            overdueTasks: [],
            todayTasks: [],
            events: [],
            openInvoices: [],
            recentClients: [],
          };
        case "finance_dashboard":
          return {
            currencies: [
              {
                currency: "USD",
                incomeMinor: 0,
                expensesMinor: 0,
                profitMinor: 0,
                unpaidMinor: 0,
                overdueMinor: 0,
                taxMinor: null,
                netMinor: null,
              },
            ],
            months: [],
            clients: [],
            openInvoices: [],
            taxesEnabled: false,
          };
        case "time_report":
          return { rows: [], total: EMPTY_TOTAL };
        case "timer_running":
        case "get_requisites":
        case "profile_image":
          return null;
        case "auto_backup_status":
          return {
            lastSuccessAt: null,
            lastAttemptAt: null,
            lastError: null,
            vaultKeyReady: false,
            keychainReady: false,
          };
        case "autostart_enabled":
          return false;
        default:
          // Lists and everything else start empty.
          return [];
      }
    },
    { shouldMockEvents: true },
  );
});

afterEach(() => {
  cleanup();
  clearMocks();
  resetSchemas();
});

const SCREENS: [string, Component][] = [
  ["Home", Home as Component],
  ["Clients", Clients as Component],
  ["New client", NewClient as Component],
  ["Projects", Projects as Component],
  ["Tasks", Tasks as Component],
  ["Calendar", Calendar as Component],
  ["Time", Time as Component],
  ["Finance", Finance as Component],
  ["Vault", Vault as Component],
  ["Settings", Settings as Component],
];

describe("screens", () => {
  it.each(SCREENS)("%s has no a11y violations", async (heading, Screen) => {
    const { container } = render(Screen);
    expect(await screen.findByRole("heading", { level: 1, name: heading })).toBeInTheDocument();
    // Let the first loads settle.
    await new Promise((r) => setTimeout(r, 50));
    await expectNoA11yViolations(container);
  });
});
