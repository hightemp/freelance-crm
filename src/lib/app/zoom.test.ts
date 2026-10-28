import { afterEach, describe, expect, it } from "vitest";
import { ZOOM_LEVELS, setZoom, stepZoom, zoom } from "./zoom.svelte";

afterEach(() => setZoom(1));

describe("UI zoom", () => {
  it("steps through levels and clamps at the ends", () => {
    stepZoom(1);
    expect(zoom.level).toBe(1.1);
    for (let i = 0; i < 20; i++) stepZoom(1);
    expect(zoom.level).toBe(ZOOM_LEVELS.at(-1));
    stepZoom(0);
    expect(zoom.level).toBe(1);
    for (let i = 0; i < 20; i++) stepZoom(-1);
    expect(zoom.level).toBe(ZOOM_LEVELS[0]);
  });

  it("applies zoom to the root element", () => {
    setZoom(1.25);
    expect(document.documentElement.style.getPropertyValue("zoom")).toBe("1.25");
  });

  it("snaps arbitrary values to the nearest level", () => {
    setZoom(1.13);
    expect(zoom.level).toBe(1.1);
  });
});
