import { render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it } from "vitest";
import DialogHost from "./DialogHost.svelte";
import { confirm, prompt, resetDialogs } from "./dialogs.svelte";

afterEach(() => resetDialogs());

describe("confirm()", () => {
  it("resolves true when confirmed", async () => {
    render(DialogHost);
    const result = confirm({ title: "Delete client?", message: "This cannot be undone." });
    const dialog = await screen.findByRole("alertdialog", { name: "Delete client?" });
    expect(dialog).toHaveAccessibleDescription("This cannot be undone.");
    await userEvent.click(screen.getByRole("button", { name: "OK" }));
    await expect(result).resolves.toBe(true);
    expect(screen.queryByRole("alertdialog")).not.toBeInTheDocument();
  });

  it("resolves false on cancel and on Escape", async () => {
    render(DialogHost);
    const first = confirm({ title: "A?", confirmLabel: "Delete", danger: true });
    await userEvent.click(await screen.findByRole("button", { name: "Cancel" }));
    await expect(first).resolves.toBe(false);

    const second = confirm({ title: "B?" });
    await screen.findByRole("alertdialog");
    await userEvent.keyboard("{Escape}");
    await expect(second).resolves.toBe(false);
  });

  it("focuses the safe (cancel) action first for dangerous confirms", async () => {
    render(DialogHost);
    void confirm({ title: "Delete?", confirmLabel: "Delete", danger: true });
    expect(await screen.findByRole("button", { name: "Cancel" })).toHaveFocus();
  });

  it("queues requests and shows them one by one", async () => {
    render(DialogHost);
    const a = confirm({ title: "First?" });
    const b = confirm({ title: "Second?" });
    await screen.findByRole("alertdialog", { name: "First?" });
    await userEvent.click(screen.getByRole("button", { name: "OK" }));
    await expect(a).resolves.toBe(true);
    await screen.findByRole("alertdialog", { name: "Second?" });
    await userEvent.click(screen.getByRole("button", { name: "Cancel" }));
    await expect(b).resolves.toBe(false);
  });
});

describe("prompt()", () => {
  it("resolves the entered value on submit with Enter", async () => {
    render(DialogHost);
    const result = prompt({ title: "Rename column", label: "Name", initial: "Todo" });
    const input = await screen.findByLabelText("Name");
    expect(input).toHaveValue("Todo");
    await userEvent.clear(input);
    await userEvent.type(input, "Backlog{Enter}");
    await expect(result).resolves.toBe("Backlog");
  });

  it("resolves null on cancel", async () => {
    render(DialogHost);
    const result = prompt({ title: "Rename", label: "Name" });
    await userEvent.click(await screen.findByRole("button", { name: "Cancel" }));
    await expect(result).resolves.toBeNull();
  });
});
