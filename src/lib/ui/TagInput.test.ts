import { render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { expectNoA11yViolations } from "../../test/a11y";
import { bind } from "../../test/bind.svelte";
import TagInput from "./TagInput.svelte";

const suggestions = ["frontend", "backend", "urgent"];

function setup(initial: string[] = []) {
  const { state, props } = bind({ label: "Tags", suggestions }, "value", initial);
  render(TagInput, props);
  return { state, input: screen.getByRole("combobox", { name: "Tags" }) };
}

describe("TagInput", () => {
  it("adds a typed tag with Enter, trimming and ignoring duplicates", async () => {
    const { state, input } = setup(["urgent"]);
    await userEvent.type(input, "  design {Enter}");
    expect(state.value).toEqual(["urgent", "design"]);
    await userEvent.type(input, "URGENT{Enter}");
    expect(state.value).toEqual(["urgent", "design"]);
    expect(input).toHaveValue("");
  });

  it("adds from suggestions and hides already chosen ones", async () => {
    const { state, input } = setup(["urgent"]);
    await userEvent.type(input, "end");
    expect(screen.getAllByRole("option").map((o) => o.textContent?.trim())).toEqual([
      "frontend",
      "backend",
    ]);
    await userEvent.click(screen.getByRole("option", { name: "backend" }));
    expect(state.value).toEqual(["urgent", "backend"]);
  });

  it("removes tags with their button and with Backspace on empty input", async () => {
    const { state, input } = setup(["a", "b", "c"]);
    await userEvent.click(screen.getByRole("button", { name: "Remove b" }));
    expect(state.value).toEqual(["a", "c"]);
    await userEvent.click(input);
    await userEvent.keyboard("{Backspace}");
    expect(state.value).toEqual(["a"]);
  });

  it("has no a11y violations", async () => {
    setup(["a"]);
    await expectNoA11yViolations(document.body);
  });
});
