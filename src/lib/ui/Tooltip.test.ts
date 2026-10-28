import { render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import TooltipProbe from "../../test/TooltipProbe.svelte";

beforeEach(() => vi.useFakeTimers());
afterEach(() => vi.useRealTimers());

const user = () => userEvent.setup({ advanceTimers: vi.advanceTimersByTime });

describe("Tooltip", () => {
  it("shows after a delay on hover and hides on leave", async () => {
    const u = user();
    render(TooltipProbe);
    const trigger = screen.getByRole("button", { name: "Copy" });
    await u.hover(trigger);
    expect(screen.queryByRole("tooltip")).not.toBeInTheDocument();
    await vi.advanceTimersByTimeAsync(500);
    const tip = screen.getByRole("tooltip");
    expect(tip).toHaveTextContent("Copy to clipboard");
    expect(trigger).toHaveAccessibleDescription("Copy to clipboard");
    await u.unhover(trigger);
    expect(screen.queryByRole("tooltip")).not.toBeInTheDocument();
  });

  it("shows on keyboard focus and hides on Escape", async () => {
    const u = user();
    render(TooltipProbe);
    await u.tab();
    await vi.advanceTimersByTimeAsync(500);
    expect(screen.getByRole("tooltip")).toBeInTheDocument();
    await u.keyboard("{Escape}");
    expect(screen.queryByRole("tooltip")).not.toBeInTheDocument();
  });
});
