import { cleanup, render, screen, waitFor } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { emit } from "@tauri-apps/api/event";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import type { VaultStatus } from "$lib/api/bindings";
import { setAppLocale } from "$lib/i18n/locale.svelte";
import { expectNoA11yViolations } from "../../../test/a11y";
import { resetVault } from "./vault.svelte";
import VaultScreen from "./VaultScreen.svelte";

let status: VaultStatus = "uninitialized";
let calls: { cmd: string; args: Record<string, unknown> }[] = [];

beforeEach(() => {
  status = "uninitialized";
  calls = [];
  resetVault();
  mockIPC(
    (cmd, args) => {
      const a = (args ?? {}) as Record<string, unknown>;
      calls.push({ cmd, args: a });
      switch (cmd) {
        case "vault_status":
          return status;
        case "vault_init":
          status = "unlocked";
          return null;
        case "vault_unlock":
          if (a.password !== "master password") {
            throw { code: "vault.wrong_password", params: {} };
          }
          status = "unlocked";
          return null;
        case "vault_lock":
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
  setAppLocale("en");
});

// Required fields end their label with "*".
const label = (name: string) => new RegExp(`^${name}\\s*\\*?$`);
const box = (name: string) => screen.getByLabelText(label(name), { selector: "input" });

describe("VaultScreen: creating the vault", () => {
  it("rejects mismatching passwords", async () => {
    render(VaultScreen);
    await userEvent.type(
      await screen.findByLabelText(label("Master password"), { selector: "input" }),
      "master password",
    );
    await userEvent.type(box("Repeat the master password"), "master passwort");
    await userEvent.click(screen.getByRole("button", { name: "Create vault" }));
    expect(box("Repeat the master password")).toHaveAccessibleDescription(
      "The passwords do not match",
    );
    expect(calls.some((c) => c.cmd === "vault_init")).toBe(false);
  });

  it("requires at least 8 characters", async () => {
    render(VaultScreen);
    await userEvent.type(
      await screen.findByLabelText(label("Master password"), { selector: "input" }),
      "short",
    );
    await userEvent.type(box("Repeat the master password"), "short");
    await userEvent.click(screen.getByRole("button", { name: "Create vault" }));
    expect(box("Master password")).toHaveAccessibleDescription(/At least 8 characters/);
  });

  it("warns about a weak password and rates a strong one", async () => {
    render(VaultScreen);
    await userEvent.type(
      await screen.findByLabelText(label("Master password"), { selector: "input" }),
      "password1",
    );
    expect(await screen.findByText(/easy to guess/)).toBeInTheDocument();
    await userEvent.clear(box("Master password"));
    await userEvent.type(box("Master password"), "violet-tractor-umbrella-sixty");
    await waitFor(() => expect(screen.queryByText(/easy to guess/)).not.toBeInTheDocument());
    expect(screen.getByRole("progressbar", { name: "Password strength" })).toHaveAttribute(
      "aria-valuenow",
      "4",
    );
  });

  it("creates the vault and opens it", async () => {
    render(VaultScreen);
    await userEvent.type(
      await screen.findByLabelText(label("Master password"), { selector: "input" }),
      "master password",
    );
    await userEvent.type(box("Repeat the master password"), "master password");
    await userEvent.click(screen.getByRole("button", { name: "Create vault" }));
    expect(await screen.findByRole("button", { name: "Lock vault" })).toBeInTheDocument();
    expect(calls.find((c) => c.cmd === "vault_init")!.args).toEqual({
      password: "master password",
    });
  });
});

describe("VaultScreen: unlocking", () => {
  beforeEach(() => {
    status = "locked";
  });

  it("shows an error and clears the field for a wrong password", async () => {
    render(VaultScreen);
    const field = await screen.findByLabelText(label("Master password"), { selector: "input" });
    await userEvent.type(field, "wrong one{Enter}");
    await waitFor(() =>
      expect(box("Master password")).toHaveAccessibleDescription("Wrong master password"),
    );
    expect(box("Master password")).toHaveValue("");
    expect(box("Master password")).toHaveFocus();
  });

  it("unlocks and locks again", async () => {
    render(VaultScreen);
    await userEvent.type(
      await screen.findByLabelText(label("Master password"), { selector: "input" }),
      "master password",
    );
    await userEvent.click(screen.getByRole("button", { name: "Unlock" }));
    await userEvent.click(await screen.findByRole("button", { name: "Lock vault" }));
    expect(await screen.findByRole("button", { name: "Unlock" })).toBeInTheDocument();
  });

  it("returns to the lock screen when the backend locks the vault", async () => {
    status = "unlocked";
    render(VaultScreen);
    await screen.findByRole("button", { name: "Lock vault" });
    status = "locked";
    await emit("vault://locked");
    expect(await screen.findByRole("button", { name: "Unlock" })).toBeInTheDocument();
  });

  it("is translated", async () => {
    setAppLocale("ru");
    render(VaultScreen);
    expect(await screen.findByRole("button", { name: "Разблокировать" })).toBeInTheDocument();
  });

  it("has no a11y violations", async () => {
    const { container } = render(VaultScreen);
    await screen.findByRole("button", { name: "Unlock" });
    await expectNoA11yViolations(container);
  });
});
