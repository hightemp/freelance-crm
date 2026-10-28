import { describe, expect, it } from "vitest";
import {
  addDays,
  addMonths,
  formatDateInput,
  formatTimeInput,
  monthGrid,
  parseDateInput,
  parseTimeInput,
  startOfWeek,
  weekdayNames,
} from "./date";

describe("monthGrid", () => {
  it("returns 6 weeks of 7 ISO dates starting on the week start (Monday)", () => {
    const grid = monthGrid(2026, 10, 1); // October 2026 starts on Thursday
    expect(grid).toHaveLength(6);
    expect(grid.every((w) => w.length === 7)).toBe(true);
    expect(grid[0][0]).toBe("2026-09-28");
    expect(grid[0][3]).toBe("2026-10-01");
    expect(grid[5][6]).toBe("2026-11-08");
  });

  it("supports Sunday as the week start", () => {
    expect(monthGrid(2026, 10, 0)[0][0]).toBe("2026-09-27");
  });

  it("handles a month starting exactly on the week start", () => {
    expect(monthGrid(2026, 6, 1)[0][0]).toBe("2026-06-01"); // June 1, 2026 is Monday
  });

  it("covers leap February", () => {
    const days = monthGrid(2028, 2, 1).flat();
    expect(days).toContain("2028-02-29");
    expect(days).not.toContain("2027-02-29");
  });
});

describe("date arithmetic", () => {
  it.each([
    ["2026-12-31", 1, "2027-01-01"],
    ["2026-03-01", -1, "2026-02-28"],
    ["2028-02-28", 1, "2028-02-29"],
    ["2026-10-25", 7, "2026-11-01"], // across DST change in Europe
  ])("addDays(%s, %d) = %s", (date, n, expected) => {
    expect(addDays(date, n)).toBe(expected);
  });

  it.each([
    ["2026-01-31", 1, "2026-02-28"],
    ["2028-01-31", 1, "2028-02-29"],
    ["2026-12-15", 1, "2027-01-15"],
    ["2026-03-31", -1, "2026-02-28"],
    ["2026-05-10", -12, "2025-05-10"],
  ])("addMonths(%s, %d) = %s", (date, n, expected) => {
    expect(addMonths(date, n)).toBe(expected);
  });

  it.each([
    ["2026-10-28", 1, "2026-10-26"],
    ["2026-10-26", 1, "2026-10-26"],
    ["2026-10-28", 0, "2026-10-25"],
  ])("startOfWeek(%s, %d) = %s", (date, weekStart, expected) => {
    expect(startOfWeek(date, weekStart)).toBe(expected);
  });
});

describe("weekdayNames", () => {
  it("orders names from the week start", () => {
    expect(weekdayNames("en", 1)[0]).toBe("Mon");
    expect(weekdayNames("en", 0)[0]).toBe("Sun");
    expect(weekdayNames("ru", 1)).toEqual(["пн", "вт", "ср", "чт", "пт", "сб", "вс"]);
  });
});

describe("parseDateInput", () => {
  it.each([
    ["28.10.2026", "ru", "2026-10-28"],
    ["1.2.26", "ru", "2026-02-01"],
    ["10/28/2026", "en", "2026-10-28"],
    ["2/1/2026", "en", "2026-02-01"],
    ["2026-10-28", "ru", "2026-10-28"],
    ["2026-10-28", "en", "2026-10-28"],
    [" 28.10.2026 ", "ru", "2026-10-28"],
    ["29.02.2028", "ru", "2028-02-29"],
  ])("%j in %s -> %s", (input, locale, expected) => {
    expect(parseDateInput(input, locale)).toBe(expected);
  });

  it.each([
    ["31.02.2026", "ru"],
    ["29.02.2026", "ru"],
    ["13/01/2026", "en"],
    ["", "en"],
    ["tomorrow", "en"],
    ["28.10", "ru"],
  ])("%j in %s -> null", (input, locale) => {
    expect(parseDateInput(input, locale)).toBeNull();
  });
});

describe("formatDateInput", () => {
  it("formats per locale and round-trips", () => {
    expect(formatDateInput("2026-02-01", "ru")).toBe("01.02.2026");
    expect(formatDateInput("2026-02-01", "en")).toBe("02/01/2026");
    for (const locale of ["en", "ru"]) {
      expect(parseDateInput(formatDateInput("2026-12-31", locale), locale)).toBe("2026-12-31");
    }
  });
});

describe("time input", () => {
  it.each([
    ["9:05", "09:05"],
    ["09:05", "09:05"],
    ["0905", "09:05"],
    ["9.30", "09:30"],
    ["9", "09:00"],
    ["23:59", "23:59"],
    ["9:05 pm", "21:05"],
    ["12:00 AM", "00:00"],
    ["12:30 pm", "12:30"],
  ])("parseTimeInput(%j) = %s", (input, expected) => {
    expect(parseTimeInput(input)).toBe(expected);
  });

  it.each(["24:00", "9:60", "", "abc", "13:00 pm"])("parseTimeInput(%j) = null", (input) => {
    expect(parseTimeInput(input)).toBeNull();
  });

  it("formats per locale clock", () => {
    expect(formatTimeInput("21:05", "ru")).toBe("21:05");
    expect(formatTimeInput("09:05", "ru")).toBe("09:05");
    expect(formatTimeInput("21:05", "en").replace(/\s/g, " ")).toBe("9:05 PM");
    expect(parseTimeInput(formatTimeInput("00:30", "en"))).toBe("00:30");
  });
});
