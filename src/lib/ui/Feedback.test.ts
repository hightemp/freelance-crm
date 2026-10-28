import { render, screen } from "@testing-library/svelte";
import { describe, expect, it } from "vitest";
import { expectNoA11yViolations } from "../../test/a11y";
import Progress from "./Progress.svelte";
import Spinner from "./Spinner.svelte";

describe("Spinner", () => {
  it("is a status with a localized label", () => {
    render(Spinner);
    expect(screen.getByRole("status", { name: "Loading" })).toBeInTheDocument();
  });
});

describe("Progress", () => {
  it("reports value within bounds", () => {
    render(Progress, { label: "Upload", value: 30, max: 120 });
    const bar = screen.getByRole("progressbar", { name: "Upload" });
    expect(bar).toHaveAttribute("aria-valuemin", "0");
    expect(bar).toHaveAttribute("aria-valuemax", "120");
    expect(bar).toHaveAttribute("aria-valuenow", "30");
  });

  it("clamps out-of-range values", () => {
    render(Progress, { label: "P", value: 500 });
    expect(screen.getByRole("progressbar")).toHaveAttribute("aria-valuenow", "100");
  });

  it("is indeterminate without a value", () => {
    render(Progress, { label: "Sync" });
    expect(screen.getByRole("progressbar")).not.toHaveAttribute("aria-valuenow");
  });

  it("has no a11y violations", async () => {
    const { container } = render(Progress, { label: "Upload", value: 10 });
    await expectNoA11yViolations(container);
  });
});
