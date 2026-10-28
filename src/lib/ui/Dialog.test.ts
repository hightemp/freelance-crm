import { render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { tick } from "svelte";
import { describe, expect, it } from "vitest";
import { expectNoA11yViolations } from "../../test/a11y";
import { bind } from "../../test/bind.svelte";
import DialogProbe from "../../test/DialogProbe.svelte";
import Dialog from "./Dialog.svelte";

describe("Dialog", () => {
  it("renders a labelled modal dialog when open", () => {
    const { props } = bind({ title: "Edit client", children: undefined }, "open", true);
    render(Dialog, props);
    const dialog = screen.getByRole("dialog", { name: "Edit client" });
    expect(dialog).toHaveAttribute("aria-modal", "true");
  });

  it("renders nothing when closed", () => {
    const { props } = bind({ title: "X" }, "open", false);
    render(Dialog, props);
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  it("traps focus and restores it to the trigger on close", async () => {
    render(DialogProbe);
    const trigger = screen.getByRole("button", { name: "Open" });
    await userEvent.click(trigger);
    const first = screen.getByLabelText("First");
    expect(first).toHaveFocus();
    await userEvent.tab();
    expect(screen.getByRole("button", { name: "Save" })).toHaveFocus();
    await userEvent.tab();
    expect(screen.getByRole("button", { name: "Close" })).toHaveFocus();
    await userEvent.tab();
    expect(first).toHaveFocus();
    await userEvent.tab({ shift: true });
    expect(screen.getByRole("button", { name: "Close" })).toHaveFocus();
    await userEvent.keyboard("{Escape}");
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    expect(trigger).toHaveFocus();
  });

  it("closes on backdrop click unless disabled", async () => {
    const { state, props } = bind({ title: "T" }, "open", true);
    render(Dialog, props);
    await userEvent.click(screen.getByTestId("dialog-backdrop"));
    expect(state.value).toBe(false);
  });

  it("keeps open on backdrop click and Escape when dismissable=false", async () => {
    const { state, props } = bind({ title: "T", dismissable: false }, "open", true);
    render(Dialog, props);
    await userEvent.click(screen.getByTestId("dialog-backdrop"));
    await userEvent.keyboard("{Escape}");
    await tick();
    expect(state.value).toBe(true);
  });

  it("has no a11y violations", async () => {
    const { props } = bind({ title: "Edit" }, "open", true);
    render(Dialog, props);
    await expectNoA11yViolations(document.body);
  });
});
