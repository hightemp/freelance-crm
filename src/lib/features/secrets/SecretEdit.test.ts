import { render, screen, waitFor } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, describe, expect, it, vi } from "vitest";
import { TEMPLATES, secret } from "./fixtures";
import SecretEdit from "./SecretEdit.svelte";

afterEach(() => clearMocks());

describe("SecretEdit", () => {
  it("loads the secret with its values and saves it", async () => {
    mockIPC((cmd) => {
      if (cmd === "get_secret" || cmd === "update_secret") return secret("s1", "Почта");
      if (cmd === "secret_payload") return { fields: { password: "old" }, custom: [] };
      if (cmd === "list_secret_templates") return TEMPLATES;
      return [];
    });
    const onsaved = vi.fn();
    render(SecretEdit, { id: "s1", onsaved, oncancel: vi.fn() });
    await waitFor(() =>
      expect(screen.getByLabelText(/^Password\s*\*?$/, { selector: "input" })).toHaveValue("old"),
    );
    await userEvent.click(screen.getByRole("button", { name: "Save" }));
    await waitFor(() => expect(onsaved).toHaveBeenCalledOnce());
  });

  it("shows a load error, e.g. a locked vault", async () => {
    mockIPC(() => {
      throw { code: "vault.locked", params: {} };
    });
    render(SecretEdit, { id: "s1", onsaved: vi.fn(), oncancel: vi.fn() });
    expect(await screen.findByRole("alert")).toHaveTextContent("The vault is locked");
  });
});
