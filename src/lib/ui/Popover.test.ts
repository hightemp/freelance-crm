import { render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import PopoverProbe from "../../test/PopoverProbe.svelte";

describe("Popover", () => {
  it("toggles from the trigger and exposes expanded state", async () => {
    render(PopoverProbe);
    const trigger = screen.getByRole("button", { name: "Filters" });
    expect(trigger).toHaveAttribute("aria-expanded", "false");
    await userEvent.click(trigger);
    expect(trigger).toHaveAttribute("aria-expanded", "true");
    const panel = screen.getByRole("dialog", { name: "Filters" });
    expect(trigger).toHaveAttribute("aria-controls", panel.id);
    await userEvent.click(trigger);
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  it("closes on Escape and returns focus to the trigger", async () => {
    render(PopoverProbe);
    const trigger = screen.getByRole("button", { name: "Filters" });
    await userEvent.click(trigger);
    await userEvent.click(screen.getByLabelText("Only active"));
    await userEvent.keyboard("{Escape}");
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    expect(trigger).toHaveFocus();
  });

  it("stays open while choosing in a select inside it", async () => {
    render(PopoverProbe);
    await userEvent.click(screen.getByRole("button", { name: "Filters" }));
    // The select's list is a separate overlay on <body>.
    await userEvent.click(screen.getByRole("combobox", { name: "Status" }));
    await userEvent.click(screen.getByRole("option", { name: "Done" }));
    expect(screen.getByRole("dialog", { name: "Filters" })).toBeInTheDocument();
    expect(screen.getByRole("combobox", { name: "Status" })).toHaveTextContent("Done");
  });

  it("closes on outside click", async () => {
    render(PopoverProbe);
    await userEvent.click(screen.getByRole("button", { name: "Filters" }));
    await userEvent.click(document.body);
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });
});
