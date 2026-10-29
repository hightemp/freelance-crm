import { cleanup, render, screen, waitFor, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import type { VaultStatus } from "$lib/api/bindings";
import DialogHost from "$lib/ui/DialogHost.svelte";
import { resetDialogs } from "$lib/ui/dialogs.svelte";
import { expectNoA11yViolations } from "../../../test/a11y";
import { resetVault } from "./vault.svelte";
import VaultSettings from "./VaultSettings.svelte";

const NEW_KEY = Array.from({ length: 14 }, () => "NNNN").join("-");
let status: VaultStatus = "unlocked";
let calls: { cmd: string; args: Record<string, unknown> }[] = [];

beforeEach(() => {
  status = "unlocked";
  calls = [];
  resetVault();
  mockIPC(
    (cmd, args) => {
      const a = (args ?? {}) as Record<string, unknown>;
      calls.push({ cmd, args: a });
      switch (cmd) {
        case "vault_status":
          return status;
        case "vault_change_password":
          if (a.oldPassword !== "master password") {
            throw { code: "vault.wrong_password", params: {} };
          }
          return null;
        case "vault_rotate_recovery_key":
          return NEW_KEY;
      }
    },
    { shouldMockEvents: true },
  );
});

afterEach(() => {
  cleanup();
  clearMocks();
  resetDialogs();
});

const label = (name: string) => new RegExp(`^${name}\\s*\\*?$`);
const box = (name: string) => screen.getByLabelText(label(name), { selector: "input" });

describe("VaultSettings", () => {
  it("asks to unlock first", async () => {
    status = "locked";
    render(VaultSettings);
    expect(
      await screen.findByText("Unlock the vault to change its password or recovery key."),
    ).toBeInTheDocument();
  });

  it("changes the master password", async () => {
    render(VaultSettings);
    await userEvent.type(
      await screen.findByLabelText(label("Current master password"), { selector: "input" }),
      "wrong",
    );
    await userEvent.type(box("New master password"), "brand new password");
    await userEvent.type(box("Repeat the new master password"), "brand new passwort");
    await userEvent.click(screen.getByRole("button", { name: "Change master password" }));
    expect(box("Repeat the new master password")).toHaveAccessibleDescription(
      "The passwords do not match",
    );

    await userEvent.clear(box("Repeat the new master password"));
    await userEvent.type(box("Repeat the new master password"), "brand new password");
    await userEvent.click(screen.getByRole("button", { name: "Change master password" }));
    await waitFor(() =>
      expect(box("Current master password")).toHaveAccessibleDescription("Wrong master password"),
    );

    await userEvent.clear(box("Current master password"));
    await userEvent.type(box("Current master password"), "master password");
    await userEvent.click(screen.getByRole("button", { name: "Change master password" }));
    await waitFor(() => expect(box("Current master password")).toHaveValue(""));
    expect(calls.findLast((c) => c.cmd === "vault_change_password")!.args).toEqual({
      oldPassword: "master password",
      newPassword: "brand new password",
    });
  });

  it("issues a new recovery key after confirmation and shows it", async () => {
    render(DialogHost);
    render(VaultSettings);
    await userEvent.click(await screen.findByRole("button", { name: "Create a new recovery key" }));
    const dialog = await screen.findByRole("alertdialog", { name: "Create a new recovery key?" });
    await userEvent.click(within(dialog).getByRole("button", { name: "Create" }));
    expect(await screen.findByRole("group", { name: "Recovery key" })).toHaveTextContent("NNNN");
  });

  it("has no a11y violations", async () => {
    const { container } = render(VaultSettings);
    await screen.findByRole("button", { name: "Create a new recovery key" });
    await expectNoA11yViolations(container);
  });
});
