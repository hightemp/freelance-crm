import { render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { expectNoA11yViolations } from "../../test/a11y";
import Checkbox from "./Checkbox.svelte";
import Switch from "./Switch.svelte";
import { bind } from "../../test/bind.svelte";

describe("Checkbox", () => {
  it("toggles on click, Space and Enter", async () => {
    const { state, props } = bind({ label: "Billable" }, "checked", false);
    render(Checkbox, props);
    const box = screen.getByRole("checkbox", { name: "Billable" });
    expect(box).toHaveAttribute("aria-checked", "false");
    await userEvent.click(box);
    expect(state.value).toBe(true);
    expect(box).toHaveAttribute("aria-checked", "true");
    await userEvent.keyboard(" ");
    expect(state.value).toBe(false);
    await userEvent.keyboard("{Enter}");
    expect(state.value).toBe(true);
  });

  it("clicking the label toggles", async () => {
    const { state, props } = bind({ label: "Done" }, "checked", false);
    render(Checkbox, props);
    await userEvent.click(screen.getByText("Done"));
    expect(state.value).toBe(true);
  });

  it("renders mixed state and resolves to checked on toggle", async () => {
    const { state, props } = bind({ label: "All", indeterminate: true }, "checked", false);
    render(Checkbox, props);
    const box = screen.getByRole("checkbox");
    expect(box).toHaveAttribute("aria-checked", "mixed");
    await userEvent.click(box);
    expect(state.value).toBe(true);
    expect(box).toHaveAttribute("aria-checked", "true");
  });

  it("does not toggle when disabled", async () => {
    const { state, props } = bind({ label: "X", disabled: true }, "checked", false);
    render(Checkbox, props);
    await userEvent.click(screen.getByRole("checkbox"));
    expect(state.value).toBe(false);
  });

  it("has no a11y violations", async () => {
    const { container } = render(Checkbox, { label: "Billable" });
    await expectNoA11yViolations(container);
  });
});

describe("Switch", () => {
  it("toggles with click, Space and Enter", async () => {
    const { state, props } = bind({ label: "Dark theme" }, "checked", false);
    render(Switch, props);
    const sw = screen.getByRole("switch", { name: "Dark theme" });
    await userEvent.click(sw);
    expect(sw).toHaveAttribute("aria-checked", "true");
    await userEvent.keyboard(" ");
    await userEvent.keyboard("{Enter}");
    expect(state.value).toBe(true);
  });

  it("has no a11y violations", async () => {
    const { container } = render(Switch, { label: "Tray" });
    await expectNoA11yViolations(container);
  });
});
