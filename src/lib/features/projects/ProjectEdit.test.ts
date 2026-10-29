import { render, screen, waitFor } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, describe, expect, it, vi } from "vitest";
import { client, project } from "./fixtures";
import ProjectEdit from "./ProjectEdit.svelte";

afterEach(() => clearMocks());

describe("ProjectEdit", () => {
  it("loads the project into the form and saves it", async () => {
    mockIPC((cmd) => {
      if (cmd === "get_project" || cmd === "update_project") return project("p1", "Сайт");
      if (cmd === "list_clients") return [client("c1", "Ромашка")];
    });
    const onsaved = vi.fn();
    render(ProjectEdit, { id: "p1", onsaved, oncancel: vi.fn() });
    expect(await screen.findByRole("textbox", { name: "Name" })).toHaveValue("Сайт");
    await userEvent.click(screen.getByRole("button", { name: "Save" }));
    await waitFor(() => expect(onsaved).toHaveBeenCalledOnce());
  });

  it("shows a load error", async () => {
    mockIPC(() => {
      throw { code: "project.not_found", params: { id: "p1" } };
    });
    render(ProjectEdit, { id: "p1", onsaved: vi.fn(), oncancel: vi.fn() });
    expect(await screen.findByRole("alert")).toHaveTextContent("Project not found");
  });
});
