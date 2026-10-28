import { render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it } from "vitest";
import { bind } from "../../test/bind.svelte";
import { setAppLocale } from "../i18n/locale.svelte";
import DatePicker from "./DatePicker.svelte";
import DateRangePicker from "./DateRangePicker.svelte";
import DateTimePicker from "./DateTimePicker.svelte";
import TimePicker from "./TimePicker.svelte";

afterEach(() => setAppLocale("en"));

describe("DatePicker", () => {
  it("parses manual input in the current locale", async () => {
    setAppLocale("ru");
    const { state, props } = bind({ label: "Deadline" }, "value", null);
    render(DatePicker, props);
    const input = screen.getByRole("textbox", { name: "Deadline" });
    await userEvent.type(input, "1.2.26");
    await userEvent.tab();
    expect(state.value).toBe("2026-02-01");
    expect(input).toHaveValue("01.02.2026");
  });

  it("shows an error for invalid and out-of-range input", async () => {
    const { state, props } = bind({ label: "Deadline", min: "2026-01-01" }, "value", "2026-05-01");
    render(DatePicker, props);
    const input = screen.getByRole("textbox");
    await userEvent.clear(input);
    await userEvent.type(input, "02/30/2026");
    await userEvent.tab();
    expect(input).toHaveAccessibleDescription("Invalid format");
    await userEvent.clear(input);
    await userEvent.type(input, "12/31/2025");
    await userEvent.tab();
    expect(input).toHaveAccessibleDescription("Out of range");
    expect(state.value).toBe("2026-05-01");
  });

  it("picks a date from the calendar popup", async () => {
    const { state, props } = bind({ label: "Deadline", today: "2026-10-28" }, "value", null);
    render(DatePicker, props);
    await userEvent.click(screen.getByRole("button", { name: "Choose date" }));
    expect(screen.getByRole("grid", { name: "October 2026" })).toBeInTheDocument();
    await userEvent.keyboard("{ArrowRight}{Enter}");
    expect(state.value).toBe("2026-10-29");
    expect(screen.queryByRole("grid")).not.toBeInTheDocument();
    expect(screen.getByRole("textbox")).toHaveValue("10/29/2026");
  });

  it("clearing the input sets null", async () => {
    const { state, props } = bind({ label: "Deadline" }, "value", "2026-10-28");
    render(DatePicker, props);
    await userEvent.clear(screen.getByRole("textbox"));
    await userEvent.tab();
    expect(state.value).toBeNull();
  });
});

describe("TimePicker", () => {
  it("parses input and steps with arrows", async () => {
    setAppLocale("ru");
    const { state, props } = bind({ label: "Start", step: 15 }, "value", null);
    render(TimePicker, props);
    const input = screen.getByRole("textbox", { name: "Start" });
    await userEvent.type(input, "930");
    await userEvent.tab();
    expect(state.value).toBe("09:30");
    expect(input).toHaveValue("09:30");
    input.focus();
    await userEvent.keyboard("{ArrowUp}");
    expect(state.value).toBe("09:45");
    await userEvent.keyboard("{ArrowDown}{ArrowDown}");
    expect(state.value).toBe("09:15");
  });

  it("wraps around midnight", async () => {
    const { state, props } = bind({ label: "Start", step: 30 }, "value", "23:45");
    render(TimePicker, props);
    screen.getByRole("textbox").focus();
    await userEvent.keyboard("{ArrowUp}");
    expect(state.value).toBe("00:15");
  });

  it("rejects invalid time", async () => {
    const { state, props } = bind({ label: "Start" }, "value", "10:00");
    render(TimePicker, props);
    const input = screen.getByRole("textbox");
    await userEvent.clear(input);
    await userEvent.type(input, "25:00");
    await userEvent.tab();
    expect(state.value).toBe("10:00");
    expect(input).toHaveAttribute("aria-invalid", "true");
  });
});

describe("DateTimePicker", () => {
  it("combines date and time into a local datetime string", async () => {
    setAppLocale("ru");
    const { state, props } = bind(
      { dateLabel: "Date", timeLabel: "Time" },
      "value",
      "2026-10-28T10:00",
    );
    render(DateTimePicker, props);
    const time = screen.getByRole("textbox", { name: "Time" });
    await userEvent.clear(time);
    await userEvent.type(time, "14:30");
    await userEvent.tab();
    expect(state.value).toBe("2026-10-28T14:30");
    const date = screen.getByRole("textbox", { name: "Date" });
    await userEvent.clear(date);
    await userEvent.type(date, "01.11.2026");
    await userEvent.tab();
    expect(state.value).toBe("2026-11-01T14:30");
  });
});

describe("DateRangePicker", () => {
  it("does not allow the end before the start", async () => {
    const state = { start: "2026-10-10" as string | null, end: "2026-10-20" as string | null };
    render(DateRangePicker, {
      startLabel: "From",
      endLabel: "To",
      get start() {
        return state.start;
      },
      set start(v) {
        state.start = v;
      },
      get end() {
        return state.end;
      },
      set end(v) {
        state.end = v;
      },
    });
    const end = screen.getByRole("textbox", { name: "To" });
    await userEvent.clear(end);
    await userEvent.type(end, "10/05/2026");
    await userEvent.tab();
    expect(state.end).toBe("2026-10-20");
    expect(end).toHaveAccessibleDescription("Out of range");

    const start = screen.getByRole("textbox", { name: "From" });
    await userEvent.clear(start);
    await userEvent.type(start, "10/25/2026");
    await userEvent.tab();
    expect(state.start).toBe("2026-10-10");
    expect(start).toHaveAccessibleDescription("Out of range");
  });
});
