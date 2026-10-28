import { render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { expectNoA11yViolations } from "../../test/a11y";
import { bind } from "../../test/bind.svelte";
import Select from "./Select.svelte";

const options = [
  { value: "lead", label: "Lead" },
  { value: "active", label: "Active" },
  { value: "paused", label: "Paused", disabled: true },
  { value: "former", label: "Former" },
];

function setup(initial: string | null = null) {
  const { state, props } = bind(
    { label: "Status", options, placeholder: "Choose…" },
    "value",
    initial,
  );
  render(Select, props);
  return { state, combo: screen.getByRole("combobox", { name: "Status" }) };
}

describe("Select", () => {
  it("shows the placeholder or the selected label", () => {
    setup();
    expect(screen.getByRole("combobox")).toHaveTextContent("Choose…");
  });

  it("opens a listbox on click with the selected option marked", async () => {
    const { combo } = setup("active");
    expect(combo).toHaveTextContent("Active");
    expect(combo).toHaveAttribute("aria-haspopup", "listbox");
    await userEvent.click(combo);
    expect(combo).toHaveAttribute("aria-expanded", "true");
    const listbox = screen.getByRole("listbox", { name: "Status" });
    expect(combo).toHaveAttribute("aria-controls", listbox.id);
    expect(screen.getByRole("option", { name: "Active" })).toHaveAttribute("aria-selected", "true");
    expect(combo).toHaveFocus();
  });

  it("navigates with aria-activedescendant, skips disabled, selects with Enter", async () => {
    const { state, combo } = setup("lead");
    combo.focus();
    await userEvent.keyboard("{ArrowDown}"); // opens on current
    const active = () => document.getElementById(combo.getAttribute("aria-activedescendant")!);
    expect(active()).toHaveTextContent("Lead");
    await userEvent.keyboard("{ArrowDown}");
    expect(active()).toHaveTextContent("Active");
    await userEvent.keyboard("{ArrowDown}");
    expect(active()).toHaveTextContent("Former");
    await userEvent.keyboard("{Home}");
    expect(active()).toHaveTextContent("Lead");
    await userEvent.keyboard("{End}{Enter}");
    expect(state.value).toBe("former");
    expect(screen.queryByRole("listbox")).not.toBeInTheDocument();
    expect(combo).toHaveFocus();
  });

  it("supports typeahead and Escape", async () => {
    const { state, combo } = setup("lead");
    combo.focus();
    await userEvent.keyboard("{Enter}");
    await userEvent.keyboard("f");
    expect(document.getElementById(combo.getAttribute("aria-activedescendant")!)).toHaveTextContent(
      "Former",
    );
    await userEvent.keyboard("{Escape}");
    expect(screen.queryByRole("listbox")).not.toBeInTheDocument();
    expect(state.value).toBe("lead");
  });

  it("selects with a click and ignores disabled options", async () => {
    const { state, combo } = setup();
    await userEvent.click(combo);
    await userEvent.click(screen.getByRole("option", { name: "Paused" }));
    expect(state.value).toBeNull();
    await userEvent.click(screen.getByRole("option", { name: "Former" }));
    expect(state.value).toBe("former");
  });

  it("has no a11y violations when open", async () => {
    const { combo } = setup();
    await userEvent.click(combo);
    await expectNoA11yViolations(document.body);
  });
});
