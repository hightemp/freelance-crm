import { describe, expect, it } from "vitest";
import { buildRule, readRule, type Repeat } from "./repeat";

const never = { kind: "never" } as const;

describe("buildRule", () => {
  it.each<[Repeat, string | null]>([
    [{ preset: "none", ends: never }, null],
    [{ preset: "daily", ends: never }, "FREQ=DAILY"],
    [{ preset: "weekdays", ends: never }, "FREQ=WEEKLY;BYDAY=MO,TU,WE,TH,FR"],
    [{ preset: "weekly", ends: never }, "FREQ=WEEKLY;BYDAY=WE"],
    [{ preset: "monthly", ends: never }, "FREQ=MONTHLY;BYMONTHDAY=18"],
    [{ preset: "yearly", ends: never }, "FREQ=YEARLY"],
    [{ preset: "weekly", ends: { kind: "count", count: 5 } }, "FREQ=WEEKLY;BYDAY=WE;COUNT=5"],
    [{ preset: "daily", ends: { kind: "until", date: "2026-04-01" } }, "FREQ=DAILY;UNTIL=20260401"],
    [
      { preset: "custom", rule: " FREQ=MONTHLY;BYDAY=-1FR ", ends: never },
      "FREQ=MONTHLY;BYDAY=-1FR",
    ],
  ])("%j", (repeat, rule) => {
    // Wednesday, March 18th.
    expect(buildRule(repeat, "2026-03-18")).toBe(rule);
  });
});

describe("readRule", () => {
  it.each<[string | null, Repeat]>([
    [null, { preset: "none", ends: never }],
    ["FREQ=DAILY", { preset: "daily", ends: never }],
    ["FREQ=WEEKLY;BYDAY=MO,TU,WE,TH,FR", { preset: "weekdays", ends: never }],
    ["FREQ=WEEKLY;BYDAY=WE;COUNT=5", { preset: "weekly", ends: { kind: "count", count: 5 } }],
    ["FREQ=WEEKLY", { preset: "weekly", ends: never }],
    ["FREQ=MONTHLY;BYMONTHDAY=18", { preset: "monthly", ends: never }],
    [
      "FREQ=YEARLY;UNTIL=20300101",
      { preset: "yearly", ends: { kind: "until", date: "2030-01-01" } },
    ],
    // Anything the presets cannot express stays a custom rule.
    ["FREQ=WEEKLY;BYDAY=MO", { preset: "custom", rule: "FREQ=WEEKLY;BYDAY=MO", ends: never }],
    ["FREQ=DAILY;INTERVAL=2", { preset: "custom", rule: "FREQ=DAILY;INTERVAL=2", ends: never }],
    [
      "FREQ=DAILY;UNTIL=20260317T065959Z",
      { preset: "custom", rule: "FREQ=DAILY;UNTIL=20260317T065959Z", ends: never },
    ],
  ])("%s", (rule, repeat) => {
    expect(readRule(rule, "2026-03-18")).toEqual(repeat);
  });

  it("round-trips the presets", () => {
    for (const rule of [
      "FREQ=DAILY;COUNT=3",
      "FREQ=WEEKLY;BYDAY=WE",
      "FREQ=YEARLY;UNTIL=20300101",
    ]) {
      expect(buildRule(readRule(rule, "2026-03-18"), "2026-03-18")).toBe(rule);
    }
  });
});
