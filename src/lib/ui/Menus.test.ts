import { fireEvent, render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { expectNoA11yViolations } from "../../test/a11y";
import MenuProbe from "../../test/MenuProbe.svelte";

function setup() {
  const onSelect = vi.fn();
  render(MenuProbe, { onSelect });
  return onSelect;
}

describe("DropdownMenu", () => {
  it("opens from the trigger and focuses the first item", async () => {
    setup();
    const trigger = screen.getByRole("button", { name: "Actions" });
    expect(trigger).toHaveAttribute("aria-haspopup", "menu");
    expect(trigger).toHaveAttribute("aria-expanded", "false");
    await userEvent.click(trigger);
    expect(trigger).toHaveAttribute("aria-expanded", "true");
    expect(screen.getByRole("menu")).toBeInTheDocument();
    expect(screen.getByRole("menuitem", { name: "Edit" })).toHaveFocus();
  });

  it("navigates with arrows, skips disabled, wraps, Home/End", async () => {
    setup();
    await userEvent.click(screen.getByRole("button", { name: "Actions" }));
    await userEvent.keyboard("{ArrowDown}");
    expect(screen.getByRole("menuitem", { name: "Duplicate" })).toHaveFocus();
    await userEvent.keyboard("{ArrowDown}");
    // "Archive" is disabled
    expect(screen.getByRole("menuitem", { name: "Delete" })).toHaveFocus();
    await userEvent.keyboard("{ArrowDown}");
    expect(screen.getByRole("menuitem", { name: "Edit" })).toHaveFocus();
    await userEvent.keyboard("{End}");
    expect(screen.getByRole("menuitem", { name: "Delete" })).toHaveFocus();
    await userEvent.keyboard("{Home}");
    expect(screen.getByRole("menuitem", { name: "Edit" })).toHaveFocus();
  });

  it("supports typeahead", async () => {
    setup();
    await userEvent.click(screen.getByRole("button", { name: "Actions" }));
    await userEvent.keyboard("d");
    expect(screen.getByRole("menuitem", { name: "Duplicate" })).toHaveFocus();
    await userEvent.keyboard("e");
    expect(screen.getByRole("menuitem", { name: "Delete" })).toHaveFocus();
  });

  it("selects with Enter, closes and returns focus to the trigger", async () => {
    const onSelect = setup();
    const trigger = screen.getByRole("button", { name: "Actions" });
    await userEvent.click(trigger);
    await userEvent.keyboard("{ArrowDown}{Enter}");
    expect(onSelect).toHaveBeenCalledWith("duplicate");
    expect(screen.queryByRole("menu")).not.toBeInTheDocument();
    expect(trigger).toHaveFocus();
  });

  it("closes on Escape without selecting", async () => {
    const onSelect = setup();
    await userEvent.click(screen.getByRole("button", { name: "Actions" }));
    await userEvent.keyboard("{Escape}");
    expect(screen.queryByRole("menu")).not.toBeInTheDocument();
    expect(onSelect).not.toHaveBeenCalled();
  });

  it("closes on outside click", async () => {
    setup();
    await userEvent.click(screen.getByRole("button", { name: "Actions" }));
    await userEvent.click(document.body);
    expect(screen.queryByRole("menu")).not.toBeInTheDocument();
  });

  it("opens with ArrowDown from the trigger", async () => {
    setup();
    screen.getByRole("button", { name: "Actions" }).focus();
    await userEvent.keyboard("{ArrowDown}");
    expect(screen.getByRole("menuitem", { name: "Edit" })).toHaveFocus();
  });

  it("has no a11y violations when open", async () => {
    setup();
    await userEvent.click(screen.getByRole("button", { name: "Actions" }));
    await expectNoA11yViolations(document.body);
  });
});

describe("ContextMenu", () => {
  it("opens on right click and selects with click", async () => {
    const onSelect = setup();
    const area = screen.getByTestId("context-area");
    await fireEvent.contextMenu(area, { clientX: 40, clientY: 50 });
    expect(screen.getByRole("menu")).toBeInTheDocument();
    await userEvent.click(screen.getByRole("menuitem", { name: "Delete" }));
    expect(onSelect).toHaveBeenCalledWith("delete");
    expect(screen.queryByRole("menu")).not.toBeInTheDocument();
  });

  it("opens with Shift+F10 and the ContextMenu key", async () => {
    setup();
    const area = screen.getByTestId("context-area");
    area.focus();
    await userEvent.keyboard("{Shift>}{F10}{/Shift}");
    expect(screen.getByRole("menu")).toBeInTheDocument();
    await userEvent.keyboard("{Escape}");
    expect(area).toHaveFocus();
    await userEvent.keyboard("{ContextMenu}");
    expect(screen.getByRole("menu")).toBeInTheDocument();
  });
});
