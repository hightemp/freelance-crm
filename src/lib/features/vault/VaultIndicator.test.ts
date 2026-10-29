import { cleanup, render, screen, waitFor } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { VaultStatus } from "$lib/api/bindings";
import { resetVault } from "./vault.svelte";
import VaultIndicator from "./VaultIndicator.svelte";

const goto = vi.hoisted(() => vi.fn());
vi.mock("$app/navigation", () => ({ goto }));

let status: VaultStatus = "unlocked";

beforeEach(() => {
  resetVault();
  mockIPC(
    (cmd) => {
      if (cmd === "vault_status") return status;
      if (cmd === "vault_lock") {
        status = "locked";
        return null;
      }
    },
    { shouldMockEvents: true },
  );
});

afterEach(() => {
  // Unmount first: components unsubscribe from mocked events on destroy.
  cleanup();
  clearMocks();
  goto.mockReset();
});

describe("VaultIndicator", () => {
  it("locks an unlocked vault", async () => {
    status = "unlocked";
    render(VaultIndicator);
    await userEvent.click(await screen.findByRole("button", { name: "Vault unlocked. Lock now" }));
    await waitFor(() =>
      expect(screen.getByRole("button", { name: "Vault locked" })).toBeInTheDocument(),
    );
  });

  it("opens the vault page otherwise", async () => {
    status = "uninitialized";
    render(VaultIndicator);
    await userEvent.click(await screen.findByRole("button", { name: "Vault not set up" }));
    expect(goto).toHaveBeenCalledWith("/vault");
  });
});
