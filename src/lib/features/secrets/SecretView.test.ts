import { cleanup, render, screen, waitFor, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { emit } from "@tauri-apps/api/event";
import { clearMocks, mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { FieldRef, SecretKind } from "$lib/api/bindings";
import DialogHost from "$lib/ui/DialogHost.svelte";
import ToastHost from "$lib/ui/ToastHost.svelte";
import { resetDialogs } from "$lib/ui/dialogs.svelte";
import { expectNoA11yViolations } from "../../../test/a11y";
import { resetVault, vault } from "../vault/vault.svelte";
import { TEMPLATES, secret } from "./fixtures";
import SecretView from "./SecretView.svelte";

const goto = vi.hoisted(() => vi.fn());
vi.mock("$app/navigation", () => ({ goto }));

const VALUES: Record<string, string> = {
  url: "https://panel.example",
  username: "admin",
  password: "hunter2",
};
let kind: SecretKind = "login";
let calls: { cmd: string; args: Record<string, unknown> }[] = [];

beforeEach(() => {
  calls = [];
  kind = "login";
  resetVault();
  vault.status = "unlocked";
  mockWindows("main");
  mockIPC(
    (cmd, args) => {
      const a = (args ?? {}) as Record<string, unknown>;
      calls.push({ cmd, args: a });
      const field = a.field as FieldRef | undefined;
      switch (cmd) {
        case "get_secret":
          return secret("s1", "Хостинг", {
            kind,
            clientId: "c1",
            clientName: "Ромашка",
            expiresOn: "2025-03-01",
          });
        case "plugin:dialog|save":
          return "/home/me/.ssh/id_deploy";
        case "export_ssh_key":
          return null;
        case "list_secret_templates":
          return TEMPLATES;
        case "secret_outline":
          return {
            fields: ["url", "username", "password"],
            custom: [{ key: "PIN", secret: true }],
          };
        case "reveal_secret_field":
          if (vault.status !== "unlocked") throw { code: "vault.locked", params: {} };
          return field?.type === "custom" ? "1234" : VALUES[field!.key];
        case "copy_secret_field":
        case "delete_secret":
          return null;
        case "list_attachments":
          return [];
        case "vault_status":
          return vault.status;
      }
    },
    { shouldMockEvents: true },
  );
});

afterEach(() => {
  cleanup();
  clearMocks();
  resetDialogs();
  goto.mockReset();
});

const reveals = () => calls.filter((c) => c.cmd === "reveal_secret_field").map((c) => c.args.field);

describe("SecretView", () => {
  it("shows plain values and masks secret ones", async () => {
    render(SecretView, { id: "s1" });
    expect(await screen.findByRole("heading", { level: 1, name: "Хостинг" })).toBeVisible();
    expect(screen.getByRole("link", { name: "Ромашка" })).toHaveAttribute("href", "/clients/c1");
    expect(screen.getByText("Renews on Mar 1, 2025")).toBeInTheDocument();
    expect(await screen.findByText("admin")).toBeInTheDocument();
    expect(screen.getByText("https://panel.example")).toBeInTheDocument();
    expect(screen.queryByText("hunter2")).not.toBeInTheDocument();
    expect(screen.getAllByText("Hidden")).toHaveLength(2);
    // Only plain values are revealed on opening.
    expect(reveals()).toEqual([
      { type: "field", key: "url" },
      { type: "field", key: "username" },
    ]);
  });

  it("reveals and hides a secret value on demand", async () => {
    render(SecretView, { id: "s1" });
    await userEvent.click(await screen.findByRole("button", { name: "Show Password" }));
    expect(await screen.findByText("hunter2")).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Hide Password" }));
    expect(screen.queryByText("hunter2")).not.toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Show PIN" }));
    expect(await screen.findByText("1234")).toBeInTheDocument();
  });

  it("saves an SSH private key to a file", async () => {
    kind = "ssh_key";
    render(ToastHost);
    render(SecretView, { id: "s1" });
    await userEvent.click(await screen.findByRole("button", { name: "Save private key to file" }));
    await waitFor(() =>
      expect(calls.find((c) => c.cmd === "export_ssh_key")?.args).toEqual({
        id: "s1",
        path: "/home/me/.ssh/id_deploy",
      }),
    );
    expect(await screen.findByText("Private key saved")).toBeInTheDocument();
  });

  it("offers saving keys only for SSH secrets", async () => {
    render(SecretView, { id: "s1" });
    await screen.findByRole("heading", { name: "Хостинг" });
    expect(
      screen.queryByRole("button", { name: "Save private key to file" }),
    ).not.toBeInTheDocument();
  });

  it("shows the secret's always-encrypted files", async () => {
    render(SecretView, { id: "s1" });
    expect(await screen.findByText("Files of secrets are always encrypted")).toBeInTheDocument();
    expect(calls.find((c) => c.cmd === "list_attachments")!.args.owner).toEqual({
      type: "secret",
      id: "s1",
    });
  });

  it("copies on the backend", async () => {
    render(SecretView, { id: "s1" });
    await userEvent.click(await screen.findByRole("button", { name: "Copy Password" }));
    await waitFor(() => expect(calls.some((c) => c.cmd === "copy_secret_field")).toBe(true));
    expect(calls.find((c) => c.cmd === "copy_secret_field")!.args).toEqual({
      id: "s1",
      field: { type: "field", key: "password" },
    });
    expect(screen.queryByText("hunter2")).not.toBeInTheDocument();
  });

  it("hides every value when the vault locks", async () => {
    render(SecretView, { id: "s1" });
    await userEvent.click(await screen.findByRole("button", { name: "Show Password" }));
    await screen.findByText("hunter2");
    vault.status = "locked";
    await emit("vault://locked");
    await waitFor(() => expect(screen.queryByText("hunter2")).not.toBeInTheDocument());
    expect(screen.queryByText("admin")).not.toBeInTheDocument();
  });

  it("edits and deletes", async () => {
    render(DialogHost);
    render(SecretView, { id: "s1" });
    await userEvent.click(await screen.findByRole("button", { name: "Edit" }));
    expect(goto).toHaveBeenCalledWith("/vault/s1/edit");
    await userEvent.click(screen.getByRole("button", { name: "Delete secret" }));
    const dialog = await screen.findByRole("alertdialog", { name: "Delete secret Хостинг?" });
    await userEvent.click(within(dialog).getByRole("button", { name: "Delete" }));
    await waitFor(() => expect(goto).toHaveBeenCalledWith("/vault"));
  });

  it("has no a11y violations", async () => {
    const { container } = render(SecretView, { id: "s1" });
    await screen.findByText("admin");
    await expectNoA11yViolations(container);
  });
});
