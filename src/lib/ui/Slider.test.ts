import { fireEvent, render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { expectNoA11yViolations } from "../../test/a11y";
import { bind } from "../../test/bind.svelte";
import Slider from "./Slider.svelte";

function setup(initial = 50) {
  const { state, props } = bind({ label: "Volume", min: 0, max: 100, step: 5 }, "value", initial);
  render(Slider, props);
  return { state, slider: screen.getByRole("slider", { name: "Volume" }) };
}

describe("Slider", () => {
  it("exposes value and bounds", () => {
    const { slider } = setup(50);
    expect(slider).toHaveAttribute("aria-valuenow", "50");
    expect(slider).toHaveAttribute("aria-valuemin", "0");
    expect(slider).toHaveAttribute("aria-valuemax", "100");
  });

  it("changes with keyboard: arrows, PageUp/Down, Home/End", async () => {
    const { state, slider } = setup(50);
    slider.focus();
    await userEvent.keyboard("{ArrowRight}");
    expect(state.value).toBe(55);
    await userEvent.keyboard("{ArrowDown}");
    expect(state.value).toBe(50);
    await userEvent.keyboard("{PageUp}");
    expect(state.value).toBe(100);
    await userEvent.keyboard("{PageDown}");
    expect(state.value).toBe(50);
    await userEvent.keyboard("{Home}");
    expect(state.value).toBe(0);
    await userEvent.keyboard("{ArrowLeft}");
    expect(state.value).toBe(0);
    await userEvent.keyboard("{End}");
    expect(state.value).toBe(100);
  });

  it("sets the value from pointer position snapped to step", async () => {
    const { state, slider } = setup(0);
    const track = slider.parentElement!;
    track.getBoundingClientRect = () => new DOMRect(0, 0, 200, 10);
    await fireEvent.pointerDown(track, { clientX: 73, pointerId: 1 });
    expect(state.value).toBe(35);
  });

  it("has no a11y violations", async () => {
    setup();
    await expectNoA11yViolations(document.body);
  });
});
