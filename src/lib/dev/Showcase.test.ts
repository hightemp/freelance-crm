import { render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import Showcase from "./Showcase.svelte";

describe("UI kit showcase", () => {
  it("renders a section for every component group", () => {
    render(Showcase);
    for (const name of [
      "Buttons",
      "Text fields",
      "Choices",
      "Pickers",
      "Dates",
      "Overlays",
      "Feedback",
    ]) {
      expect(screen.getByRole("heading", { name })).toBeInTheDocument();
    }
  });

  it("shows disabled, error and loading states", () => {
    render(Showcase);
    expect(screen.getAllByRole("button", { name: /Disabled/ })[0]).toBeDisabled();
    expect(screen.getByLabelText("With error")).toHaveAttribute("aria-invalid", "true");
    expect(screen.getAllByRole("button", { busy: true }).length).toBeGreaterThan(0);
  });

  it("toggles the preview theme", async () => {
    render(Showcase);
    await userEvent.click(screen.getByRole("switch", { name: "Dark preview" }));
    expect(document.documentElement.dataset.theme).toBe("dark");
  });
});
