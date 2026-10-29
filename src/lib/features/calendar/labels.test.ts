import { describe, expect, it } from "vitest";
import { reminderLabel, shiftLocal } from "./labels";

describe("reminderLabel", () => {
  it.each([
    [0, "At the start"],
    [15, "15 min before"],
    [90, "90 min before"],
    [120, "2 h before"],
    [2880, "2 d before"],
    [10080, "1 wk before"],
  ])("%d → %s", (minutes, label) => {
    expect(reminderLabel(minutes)).toBe(label);
  });
});

describe("shiftLocal", () => {
  it("moves a series start by an occurrence's change", () => {
    // The March 16th occurrence moved from 10:00 to 11:30 on the 17th.
    expect(shiftLocal("2026-03-02T10:00", "2026-03-16T10:00", "2026-03-17T11:30")).toBe(
      "2026-03-03T11:30",
    );
    expect(shiftLocal("2026-03-02", "2026-03-16", "2026-03-18")).toBe("2026-03-04");
  });
});
