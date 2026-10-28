import { render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import EditMenuHost from "./EditMenuHost.svelte";
import { openEditMenu } from "./edit-menu.svelte";

describe("EditMenuHost", () => {
  it("shows cut/copy/paste/select all for an input and runs the command", async () => {
    const exec = vi.fn(() => true);
    document.execCommand = exec;
    render(EditMenuHost);
    const input = document.body.appendChild(document.createElement("input"));
    input.value = "hello";
    input.focus();
    openEditMenu({ x: 5, y: 5, target: input });
    const items = await screen.findAllByRole("menuitem");
    expect(items.map((i) => i.textContent?.trim())).toEqual(["Cut", "Copy", "Paste", "Select all"]);
    await userEvent.click(screen.getByRole("menuitem", { name: "Copy" }));
    expect(exec).toHaveBeenCalledWith("copy");
    expect(screen.queryByRole("menu")).not.toBeInTheDocument();
  });

  it("disables cut and paste for read-only fields", async () => {
    render(EditMenuHost);
    const input = document.body.appendChild(document.createElement("input"));
    input.readOnly = true;
    openEditMenu({ x: 5, y: 5, target: input });
    expect(await screen.findByRole("menuitem", { name: "Cut" })).toBeDisabled();
    expect(screen.getByRole("menuitem", { name: "Paste" })).toBeDisabled();
    expect(screen.getByRole("menuitem", { name: "Copy" })).toBeEnabled();
  });
});
