import { render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { expectNoA11yViolations } from "../../test/a11y";
import { text } from "../../test/snippets.svelte";
import Button from "./Button.svelte";

describe("Button", () => {
  it("renders a type=button by default and handles clicks", async () => {
    const onclick = vi.fn();
    render(Button, { children: text("Save"), onclick });
    const button = screen.getByRole("button", { name: "Save" });
    expect(button).toHaveAttribute("type", "button");
    await userEvent.click(button);
    expect(onclick).toHaveBeenCalledOnce();
  });

  it("is keyboard operable with Enter and Space", async () => {
    const onclick = vi.fn();
    render(Button, { children: text("Go"), onclick });
    screen.getByRole("button").focus();
    await userEvent.keyboard("{Enter}");
    await userEvent.keyboard(" ");
    expect(onclick).toHaveBeenCalledTimes(2);
  });

  it("does not fire when disabled", async () => {
    const onclick = vi.fn();
    render(Button, { children: text("Go"), onclick, disabled: true });
    await userEvent.click(screen.getByRole("button"));
    expect(onclick).not.toHaveBeenCalled();
  });

  it("shows a spinner and blocks clicks while loading", async () => {
    const onclick = vi.fn();
    render(Button, { children: text("Go"), onclick, loading: true });
    const button = screen.getByRole("button");
    expect(button).toHaveAttribute("aria-busy", "true");
    expect(screen.getByRole("status")).toBeInTheDocument();
    await userEvent.click(button);
    expect(onclick).not.toHaveBeenCalled();
  });

  it("applies the variant", () => {
    render(Button, { children: text("Delete"), variant: "danger" });
    expect(screen.getByRole("button")).toHaveAttribute("data-variant", "danger");
  });

  it("has no a11y violations", async () => {
    const { container } = render(Button, { children: text("Save") });
    await expectNoA11yViolations(container);
  });
});
