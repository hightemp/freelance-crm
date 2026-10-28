import { render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { expectNoA11yViolations } from "../../test/a11y";
import { bind } from "../../test/bind.svelte";
import RadioGroup from "./RadioGroup.svelte";

const options = [
  { value: "person", label: "Person" },
  { value: "company", label: "Company" },
  { value: "other", label: "Other", disabled: true },
  { value: "gov", label: "Government" },
];

function setup(initial = "person") {
  const { state, props } = bind(
    {
      label: "Client type",
      options,
    },
    "value",
    initial,
  );
  render(RadioGroup, props);
  return state;
}

describe("RadioGroup", () => {
  it("exposes a labelled radiogroup with the selected radio", () => {
    setup();
    expect(screen.getByRole("radiogroup", { name: "Client type" })).toBeInTheDocument();
    expect(screen.getByRole("radio", { name: "Person" })).toHaveAttribute("aria-checked", "true");
    expect(screen.getByRole("radio", { name: "Company" })).toHaveAttribute("aria-checked", "false");
  });

  it("uses roving tabindex: only the selected radio is tabbable", () => {
    setup("company");
    const tabbable = screen.getAllByRole("radio").filter((r) => r.tabIndex === 0);
    expect(tabbable.map((r) => r.textContent?.trim())).toEqual(["Company"]);
  });

  it("arrow keys move selection, skip disabled and wrap", async () => {
    const state = setup("company");
    screen.getByRole("radio", { name: "Company" }).focus();
    await userEvent.keyboard("{ArrowDown}");
    expect(state.value).toBe("gov");
    expect(screen.getByRole("radio", { name: "Government" })).toHaveFocus();
    await userEvent.keyboard("{ArrowRight}");
    expect(state.value).toBe("person");
    await userEvent.keyboard("{ArrowUp}");
    expect(state.value).toBe("gov");
    await userEvent.keyboard("{ArrowLeft}");
    expect(state.value).toBe("company");
  });

  it("selects on click but not when disabled", async () => {
    const state = setup();
    await userEvent.click(screen.getByRole("radio", { name: "Company" }));
    expect(state.value).toBe("company");
    await userEvent.click(screen.getByRole("radio", { name: "Other" }));
    expect(state.value).toBe("company");
  });

  it("has no a11y violations", async () => {
    setup();
    await expectNoA11yViolations(document.body);
  });
});
