import { cleanup, render, screen, waitFor, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { SecretInput_Deserialize as SecretInput } from "$lib/api/bindings";
import { expectNoA11yViolations } from "../../../test/a11y";
import { client, project } from "../projects/fixtures";
import { TEMPLATES, secret } from "./fixtures";
import SecretForm from "./SecretForm.svelte";

let calls: { cmd: string; args: Record<string, unknown> }[] = [];

beforeEach(() => {
  calls = [];
  mockIPC((cmd, args) => {
    const a = (args ?? {}) as Record<string, unknown>;
    calls.push({ cmd, args: a });
    switch (cmd) {
      case "list_secret_templates":
        return TEMPLATES;
      case "list_clients":
        return [client("c1", "Ромашка")];
      case "list_projects":
        return [project("p1", "Сайт")];
      case "generate_password":
        return { value: "Gen3rated!Pass", entropyBits: 104.5 };
      case "generate_passphrase":
        return { value: "acid-acorn-acre-zone-zoom", entropyBits: 51.7 };
      case "generate_ssh_key":
        return {
          privateKey:
            "-----BEGIN OPENSSH PRIVATE KEY-----\nAAA\n-----END OPENSSH PRIVATE KEY-----\n",
          publicKey: "ssh-ed25519 AAAA deploy",
          algorithm: "ssh-ed25519",
          comment: "deploy",
          fingerprint: "SHA256:x",
          encrypted: false,
        };
      case "create_secret":
      case "update_secret":
        return secret((a.id as string) ?? "s-new", (a.input as SecretInput).title);
    }
  });
});

afterEach(() => {
  cleanup();
  clearMocks();
});

const label = (name: string) => new RegExp(`^${name}\\s*\\*?$`);
const box = (name: string) => screen.getByLabelText(label(name), { selector: "input, textarea" });
const sent = (cmd: string) => calls.find((c) => c.cmd === cmd)!.args;

async function choose(name: string, option: string) {
  await userEvent.click(screen.getByRole("combobox", { name }));
  await userEvent.click(screen.getByRole("option", { name: option }));
}

async function ready() {
  await waitFor(() =>
    expect(screen.getByRole("combobox", { name: "Type" })).toHaveTextContent("Login"),
  );
}

describe("SecretForm", () => {
  it("rebuilds the fields when the type changes", async () => {
    render(SecretForm, { onsaved: vi.fn() });
    await ready();
    expect(box("Username")).toBeInTheDocument();
    expect(box("Password")).toHaveAttribute("type", "password");
    await choose("Type", "SSH key");
    expect(box("Private key").tagName).toBe("TEXTAREA");
    expect(screen.queryByLabelText(label("Username"))).not.toBeInTheDocument();
  });

  it("inserts a generated password into the field", async () => {
    render(SecretForm, { onsaved: vi.fn() });
    await ready();
    await userEvent.click(screen.getByRole("button", { name: "Generate Password" }));
    const dialog = await screen.findByRole("dialog", { name: "Password generator" });
    expect(within(dialog).getByText("Gen3rated!Pass")).toBeInTheDocument();
    await userEvent.click(within(dialog).getByRole("radio", { name: "Passphrase" }));
    expect(await within(dialog).findByText("acid-acorn-acre-zone-zoom")).toBeInTheDocument();
    await userEvent.click(within(dialog).getByRole("button", { name: "Use" }));
    expect(box("Password")).toHaveValue("acid-acorn-acre-zone-zoom");
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  it("fills an SSH secret from a generated key pair", async () => {
    render(SecretForm, { onsaved: vi.fn() });
    await ready();
    await choose("Type", "SSH key");
    await userEvent.click(screen.getByRole("button", { name: "Generate key pair" }));
    const dialog = await screen.findByRole("dialog", { name: "Generate an SSH key" });
    await userEvent.type(
      within(dialog).getByLabelText(label("Comment"), { selector: "input" }),
      "deploy",
    );
    await userEvent.click(within(dialog).getByRole("button", { name: "Generate key pair" }));
    await waitFor(() => expect(box("Public key")).toHaveValue("ssh-ed25519 AAAA deploy"));
    expect((box("Private key") as HTMLTextAreaElement).value).toContain("OPENSSH PRIVATE KEY");
    expect(sent("generate_ssh_key")).toEqual({
      keyType: "ed25519",
      comment: "deploy",
      passphrase: null,
    });
  });

  it("saves a new secret with its values and extra fields", async () => {
    const onsaved = vi.fn();
    render(SecretForm, { clientId: "c1", onsaved });
    await ready();
    await userEvent.type(box("Title"), "Хостинг");
    await userEvent.type(box("Username"), "admin");
    await userEvent.type(box("Password"), "hunter2");
    await userEvent.click(screen.getByRole("button", { name: "Add field" }));
    await userEvent.type(box("Name"), "PIN");
    await userEvent.type(box("Value"), "1234");
    await userEvent.click(screen.getByRole("button", { name: "Save" }));
    await waitFor(() => expect(onsaved).toHaveBeenCalledOnce());
    expect(sent("create_secret").input).toEqual({
      title: "Хостинг",
      kind: "login",
      clientId: "c1",
      projectId: null,
      expiresOn: null,
      payload: {
        fields: { username: "admin", password: "hunter2" },
        custom: [{ key: "PIN", value: "1234", secret: true }],
      },
    });
  });

  it("drops values of fields the new type does not have", async () => {
    render(SecretForm, { onsaved: vi.fn() });
    await ready();
    await userEvent.type(box("Title"), "ENV");
    await userEvent.type(box("Username"), "admin");
    await choose("Type", ".env file");
    await userEvent.type(box("Content"), "A=1");
    await userEvent.click(screen.getByRole("button", { name: "Save" }));
    await waitFor(() => expect(calls.some((c) => c.cmd === "create_secret")).toBe(true));
    expect((sent("create_secret").input as SecretInput).payload.fields).toEqual({ content: "A=1" });
  });

  it("requires the title and required fields", async () => {
    render(SecretForm, { onsaved: vi.fn() });
    await ready();
    await userEvent.click(screen.getByRole("button", { name: "Save" }));
    expect(box("Title")).toHaveAccessibleDescription("This field is required");
    expect(box("Password")).toHaveAccessibleDescription("This field is required");
    expect(calls.some((c) => c.cmd === "create_secret")).toBe(false);
  });

  it("edits an existing secret with its values", async () => {
    const onsaved = vi.fn();
    render(SecretForm, {
      secret: secret("s1", "Почта", { clientId: "c1", clientName: "Ромашка" }),
      payload: { fields: { username: "me", password: "old" }, custom: [] },
      onsaved,
    });
    // Fields appear once the templates are loaded.
    await waitFor(() => expect(box("Password")).toHaveValue("old"));
    expect(box("Title")).toHaveValue("Почта");
    await userEvent.clear(box("Password"));
    await userEvent.type(box("Password"), "new");
    await userEvent.click(screen.getByRole("button", { name: "Save" }));
    await waitFor(() => expect(onsaved).toHaveBeenCalledOnce());
    expect(sent("update_secret")).toMatchObject({
      id: "s1",
      input: { clientId: "c1", payload: { fields: { username: "me", password: "new" } } },
    });
  });

  it("has no a11y violations", async () => {
    const { container } = render(SecretForm, { onsaved: vi.fn() });
    await ready();
    await expectNoA11yViolations(container);
  });
});
