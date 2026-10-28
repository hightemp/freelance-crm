import { render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { expectNoA11yViolations } from "../../test/a11y";
import CalendarGrid from "./CalendarGrid.svelte";

function setup(props: Record<string, unknown> = {}) {
  const onselect = vi.fn();
  render(CalendarGrid, {
    selected: "2026-10-28",
    today: "2026-10-28",
    weekStart: 1,
    autofocus: true,
    onselect,
    ...props,
  });
  return onselect;
}

const focusedDay = () => (document.activeElement as HTMLElement).dataset.date;

describe("CalendarGrid", () => {
  it("renders a labelled grid with localized weekday headers", () => {
    setup();
    const grid = screen.getByRole("grid", { name: "October 2026" });
    expect(grid).toBeInTheDocument();
    expect(screen.getAllByRole("columnheader").map((h) => h.textContent?.trim())[0]).toBe("Mon");
  });

  it("focuses the selected day and marks it selected and current", async () => {
    setup();
    await vi.waitFor(() => expect(focusedDay()).toBe("2026-10-28"));
    const cell = screen.getByRole("gridcell", { selected: true });
    expect(cell).toHaveTextContent("28");
    expect(document.activeElement).toHaveAttribute("aria-current", "date");
  });

  it("moves focus with arrows, Home/End and pages months", async () => {
    setup();
    await userEvent.keyboard("{ArrowRight}");
    expect(focusedDay()).toBe("2026-10-29");
    await userEvent.keyboard("{ArrowDown}");
    expect(focusedDay()).toBe("2026-11-05");
    expect(screen.getByRole("grid", { name: "November 2026" })).toBeInTheDocument();
    await userEvent.keyboard("{Home}");
    expect(focusedDay()).toBe("2026-11-02");
    await userEvent.keyboard("{End}");
    expect(focusedDay()).toBe("2026-11-08");
    await userEvent.keyboard("{PageUp}");
    expect(focusedDay()).toBe("2026-10-08");
    await userEvent.keyboard("{Shift>}{PageDown}{/Shift}");
    expect(focusedDay()).toBe("2027-10-08");
  });

  it("selects with Enter and click", async () => {
    const onselect = setup();
    await userEvent.keyboard("{ArrowLeft}{Enter}");
    expect(onselect).toHaveBeenLastCalledWith("2026-10-27");
    await userEvent.click(screen.getByRole("button", { name: /October 15, 2026/ }));
    expect(onselect).toHaveBeenLastCalledWith("2026-10-15");
  });

  it("disables dates outside min/max and keeps focus within range", async () => {
    const onselect = setup({ min: "2026-10-27", max: "2026-10-29" });
    expect(screen.getByRole("button", { name: /October 26, 2026/ })).toHaveAttribute(
      "aria-disabled",
      "true",
    );
    await userEvent.click(screen.getByRole("button", { name: /October 26, 2026/ }));
    expect(onselect).not.toHaveBeenCalled();
    await userEvent.keyboard("{ArrowRight}{ArrowRight}{ArrowRight}");
    expect(focusedDay()).toBe("2026-10-29");
  });

  it("switches months with the navigation buttons", async () => {
    setup();
    await userEvent.click(screen.getByRole("button", { name: "Next month" }));
    expect(screen.getByRole("grid", { name: "November 2026" })).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Previous month" }));
    await userEvent.click(screen.getByRole("button", { name: "Previous month" }));
    expect(screen.getByRole("grid", { name: "September 2026" })).toBeInTheDocument();
  });

  it("has no a11y violations", async () => {
    const { container } = render(CalendarGrid, {
      selected: null,
      today: "2026-10-28",
      weekStart: 1,
      onselect: vi.fn(),
    });
    await expectNoA11yViolations(container);
  });
});
