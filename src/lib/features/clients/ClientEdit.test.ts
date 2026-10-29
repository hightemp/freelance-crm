import { render, screen, waitFor } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, describe, expect, it, vi } from "vitest";
import ClientEdit from "./ClientEdit.svelte";

const client = {
  id: "c1",
  kind: "company",
  lastName: null,
  firstName: null,
  middleName: null,
  companyName: "Ромашка",
  legalName: null,
  status: "active",
  defaultCurrency: null,
  defaultRateMinor: null,
  displayName: "Ромашка",
  tagIds: [],
  createdAt: 0,
  updatedAt: 0,
  archivedAt: null,
};

afterEach(() => clearMocks());

describe("ClientEdit", () => {
  it("loads the client into the form and saves it", async () => {
    mockIPC((cmd, args) => {
      if (cmd === "get_client") return client;
      if (cmd === "list_contact_methods") return [];
      if (cmd === "update_client") return { ...client, ...(args as { input: object }).input };
      if (cmd === "replace_contact_methods") return [];
    });
    const onsaved = vi.fn();
    render(ClientEdit, { id: "c1", onsaved, oncancel: vi.fn() });
    expect(await screen.findByRole("textbox", { name: "Short name" })).toHaveValue("Ромашка");
    await userEvent.click(screen.getByRole("button", { name: "Save" }));
    await waitFor(() => expect(onsaved).toHaveBeenCalledOnce());
  });

  it("shows a load error", async () => {
    mockIPC(() => {
      throw { code: "client.not_found", params: { id: "c1" } };
    });
    render(ClientEdit, { id: "c1", onsaved: vi.fn(), oncancel: vi.fn() });
    expect(await screen.findByRole("alert")).toHaveTextContent("Client not found");
  });
});
