import { render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { expectNoA11yViolations } from "../../test/a11y";
import { bind } from "../../test/bind.svelte";
import Combobox from "./Combobox.svelte";

const clients = [
  { value: "1", label: "ООО Ромашка" },
  { value: "2", label: "Acme Corp" },
  { value: "3", label: "Иванов Иван" },
];

describe("Combobox", () => {
  it("filters options as you type, case-insensitive", async () => {
    const { props } = bind({ label: "Client", options: clients }, "value", null);
    render(Combobox, props);
    const input = screen.getByRole("combobox", { name: "Client" });
    expect(input).toHaveAttribute("aria-autocomplete", "list");
    await userEvent.type(input, "ром");
    expect(input).toHaveAttribute("aria-expanded", "true");
    expect(screen.getAllByRole("option").map((o) => o.textContent?.trim())).toEqual([
      "ООО Ромашка",
    ]);
  });

  it("selects with arrows and Enter, showing the label in the input", async () => {
    const { state, props } = bind({ label: "Client", options: clients }, "value", null);
    render(Combobox, props);
    const input = screen.getByRole("combobox");
    await userEvent.click(input);
    await userEvent.keyboard("{ArrowDown}{ArrowDown}");
    expect(document.getElementById(input.getAttribute("aria-activedescendant")!)).toHaveTextContent(
      "Acme Corp",
    );
    await userEvent.keyboard("{Enter}");
    expect(state.value).toBe("2");
    expect(input).toHaveValue("Acme Corp");
    expect(screen.queryByRole("listbox")).not.toBeInTheDocument();
  });

  it("loads options asynchronously", async () => {
    const load = vi.fn(async (q: string) =>
      clients.filter((c) => c.label.toLowerCase().includes(q.toLowerCase())),
    );
    const { props } = bind({ label: "Client", load }, "value", null);
    render(Combobox, props);
    await userEvent.type(screen.getByRole("combobox"), "acme");
    expect(await screen.findByRole("option", { name: "Acme Corp" })).toBeInTheDocument();
    expect(load).toHaveBeenLastCalledWith("acme");
  });

  it("offers to create a new value when nothing matches exactly", async () => {
    const onCreate = vi.fn(async (label: string) => ({ value: "new", label }));
    const { state, props } = bind({ label: "Tag", options: clients, onCreate }, "value", null);
    render(Combobox, props);
    await userEvent.type(screen.getByRole("combobox"), "Urgent");
    const create = screen.getByRole("option", { name: "Create “Urgent”" });
    await userEvent.click(create);
    expect(onCreate).toHaveBeenCalledWith("Urgent");
    await vi.waitFor(() => expect(state.value).toBe("new"));
  });

  it("restores the selected label on Escape", async () => {
    const { props } = bind({ label: "Client", options: clients }, "value", "2");
    render(Combobox, props);
    const input = screen.getByRole("combobox");
    expect(input).toHaveValue("Acme Corp");
    await userEvent.clear(input);
    await userEvent.type(input, "zzz{Escape}");
    expect(input).toHaveValue("Acme Corp");
  });

  it("has no a11y violations when open", async () => {
    const { props } = bind({ label: "Client", options: clients }, "value", null);
    render(Combobox, props);
    await userEvent.click(screen.getByRole("combobox"));
    await userEvent.keyboard("{ArrowDown}");
    await expectNoA11yViolations(document.body);
  });
});
