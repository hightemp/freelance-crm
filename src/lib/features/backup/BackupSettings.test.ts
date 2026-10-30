import { cleanup, render, screen, waitFor, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { appSettings, DEFAULT_SETTINGS } from "$lib/app/settings.svelte";
import ToastHost from "$lib/ui/ToastHost.svelte";
import { expectNoA11yViolations } from "../../../test/a11y";
import BackupSettings from "./BackupSettings.svelte";

let calls: { cmd: string; args: Record<string, unknown> }[] = [];
let kind = "passphrase";
let status: Record<string, unknown>;

beforeEach(() => {
  calls = [];
  kind = "passphrase";
  status = {
    lastSuccessAt: null,
    lastAttemptAt: null,
    lastError: null,
    vaultKeyReady: false,
    keychainReady: false,
  };
  appSettings.value = { ...DEFAULT_SETTINGS };
  mockIPC((cmd, args) => {
    const a = (args ?? {}) as Record<string, unknown>;
    calls.push({ cmd, args: a });
    switch (cmd) {
      case "auto_backup_status":
        return status;
      case "plugin:dialog|save":
        return "/home/me/crm.fcrm-backup";
      case "plugin:dialog|open":
        return (a.options as { directory?: boolean })?.directory
          ? "/home/me/Backups"
          : "/home/me/old.fcrm-backup";
      case "export_backup":
        return { files: [{}, {}] };
      case "inspect_backup":
        return kind;
      case "restore_backup":
        return { manifest: {}, safetyBackup: "/data/backups/before-restore.fcrm-backup" };
      case "save_settings":
        return a.settings;
      case "set_backup_passphrase":
      case "set_up_vault_backup_key":
        return null;
      case "export_data":
        return ["freelance-crm.json", "clients.csv"];
    }
  });
});

afterEach(() => {
  cleanup();
  clearMocks();
});

function setup() {
  const onrestored = vi.fn();
  render(BackupSettings, { onrestored });
  render(ToastHost);
  return { onrestored, user: userEvent.setup() };
}

describe("BackupSettings", () => {
  it("creates a backup after the password is typed twice", async () => {
    const { user } = setup();
    await user.click(screen.getByRole("button", { name: "Create backup…" }));
    const dialog = screen.getByRole("dialog", { name: "Create backup" });
    await user.type(within(dialog).getByLabelText(/^Backup password/), "correct horse");
    await user.type(within(dialog).getByLabelText(/^Repeat the password/), "correct hose");
    await user.click(within(dialog).getByRole("button", { name: "Choose file and save" }));
    expect(within(dialog).getByText("The passwords do not match")).toBeInTheDocument();
    expect(calls.some((c) => c.cmd === "export_backup")).toBe(false);

    const repeat = within(dialog).getByLabelText(/^Repeat the password/);
    await user.clear(repeat);
    await user.type(repeat, "correct horse");
    await user.click(within(dialog).getByRole("button", { name: "Choose file and save" }));
    await waitFor(() =>
      expect(calls.find((c) => c.cmd === "export_backup")?.args).toEqual({
        path: "/home/me/crm.fcrm-backup",
        passphrase: "correct horse",
      }),
    );
    expect(await screen.findByText("Backup saved")).toBeInTheDocument();
  });

  it("warns before restoring and asks for what the file needs", async () => {
    kind = "vault";
    const { user, onrestored } = setup();
    await user.click(screen.getByRole("button", { name: "Restore from backup…" }));
    const dialog = await screen.findByRole("alertdialog", { name: "Restore from backup" });
    expect(dialog).toHaveTextContent("replaces all current data");
    expect(dialog).toHaveTextContent("old.fcrm-backup");
    await user.type(within(dialog).getByLabelText(/^Master password/), "master password");
    await user.click(within(dialog).getByRole("button", { name: "Replace my data" }));
    await waitFor(() =>
      expect(calls.find((c) => c.cmd === "restore_backup")?.args).toEqual({
        path: "/home/me/old.fcrm-backup",
        secret: "master password",
      }),
    );
    await waitFor(() => expect(onrestored).toHaveBeenCalled());
  });

  it("saves the automatic backup settings", async () => {
    const { user } = setup();
    const schedule = screen.getByRole("combobox", { name: "Automatic backups" });
    expect(schedule).toBeDisabled();
    await user.click(screen.getByRole("button", { name: "Choose folder…" }));
    await waitFor(() => expect(appSettings.value.autoBackupDir).toBe("/home/me/Backups"));
    await user.click(schedule);
    await user.click(screen.getByRole("option", { name: "Daily" }));
    await waitFor(() => expect(appSettings.value.autoBackupSchedule).toBe("daily"));

    await user.click(screen.getByRole("radio", { name: /keychain/ }));
    await waitFor(() => expect(appSettings.value.autoBackupKey).toBe("keychain"));
    await user.type(screen.getByLabelText(/^Password for automatic backups/), "long enough");
    await user.click(screen.getByRole("button", { name: "Save to keychain" }));
    await waitFor(() =>
      expect(calls.find((c) => c.cmd === "set_backup_passphrase")?.args).toEqual({
        passphrase: "long enough",
      }),
    );
    const saved = calls.filter((c) => c.cmd === "save_settings").at(-1);
    expect(saved?.args.settings).toMatchObject({
      autoBackupDir: "/home/me/Backups",
      autoBackupSchedule: "daily",
      autoBackupKey: "keychain",
      autoBackupKeep: 7,
    });
  });

  it("sets up vault-locked backups and shows the last result", async () => {
    status = { ...status, lastError: "backup.folder_unavailable", lastAttemptAt: 1 };
    const { user } = setup();
    expect(await screen.findByText(/The backup folder is unavailable/)).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Set up" }));
    await waitFor(() => expect(calls.some((c) => c.cmd === "set_up_vault_backup_key")).toBe(true));
  });

  it("exports the data as JSON and CSV", async () => {
    const { user } = setup();
    await user.click(screen.getByRole("button", { name: "Export data (JSON, CSV)…" }));
    await waitFor(() =>
      expect(calls.find((c) => c.cmd === "export_data")?.args).toEqual({ dir: "/home/me/Backups" }),
    );
    expect(await screen.findByText("Exported 2 files")).toBeInTheDocument();
  });

  it("has no a11y violations", async () => {
    setup();
    await screen.findByRole("button", { name: "Create backup…" });
    await expectNoA11yViolations(document.body);
  });
});
