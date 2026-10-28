import { render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { bind } from "../../test/bind.svelte";
import ColorPicker from "./ColorPicker.svelte";
import { normalizeHex } from "./color";

describe("normalizeHex", () => {
  it.each([
    ["#3B5BDB", "#3b5bdb"],
    ["3b5bdb", "#3b5bdb"],
    ["#abc", "#aabbcc"],
    [" #ABC ", "#aabbcc"],
    ["#12345", null],
    ["red", null],
    ["#gggggg", null],
  ])("%j -> %s", (input, expected) => {
    expect(normalizeHex(input)).toBe(expected);
  });
});

describe("ColorPicker", () => {
  it("picks a preset swatch", async () => {
    const { state, props } = bind({ label: "Tag color" }, "value", "#3b5bdb");
    render(ColorPicker, props);
    const trigger = screen.getByRole("button", { name: /Tag color/ });
    await userEvent.click(trigger);
    const swatch = screen.getAllByRole("radio")[1];
    await userEvent.click(swatch);
    expect(state.value).toMatch(/^#[0-9a-f]{6}$/);
    expect(state.value).toBe(swatch.getAttribute("aria-label"));
  });

  it("accepts a valid hex and rejects an invalid one", async () => {
    const { state, props } = bind({ label: "Tag color" }, "value", "#3b5bdb");
    render(ColorPicker, props);
    await userEvent.click(screen.getByRole("button", { name: /Tag color/ }));
    const hex = screen.getByLabelText("HEX");
    await userEvent.clear(hex);
    await userEvent.type(hex, "nope{Enter}");
    expect(state.value).toBe("#3b5bdb");
    expect(hex).toHaveAttribute("aria-invalid", "true");
    await userEvent.clear(hex);
    await userEvent.type(hex, "#F80{Enter}");
    expect(state.value).toBe("#ff8800");
  });
});
