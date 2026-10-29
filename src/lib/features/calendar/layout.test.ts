import { describe, expect, it } from "vitest";
import { hiddenPerDay, layoutDay, weekSegments } from "./layout";

const span = (id: string, start: number, end: number) => ({ id, start, end });

describe("layoutDay", () => {
  it.each([
    ["no overlap: full width", [span("a", 60, 120), span("b", 120, 180)], { a: [0, 1], b: [0, 1] }],
    [
      "two overlapping: halves",
      [span("a", 60, 180), span("b", 120, 240)],
      { a: [0, 2], b: [1, 2] },
    ],
    [
      "a long one with two short ones beside it",
      [span("long", 0, 240), span("x", 30, 90), span("y", 120, 180)],
      { long: [0, 2], x: [1, 2], y: [1, 2] },
    ],
    [
      "three at once, then a lone one",
      [span("a", 0, 60), span("b", 0, 60), span("c", 30, 90), span("d", 200, 260)],
      { a: [0, 3], b: [1, 3], c: [2, 3], d: [0, 1] },
    ],
    [
      "a freed column is reused within a cluster",
      [span("a", 0, 120), span("b", 0, 30), span("c", 60, 90)],
      { a: [0, 2], b: [1, 2], c: [1, 2] },
    ],
    [
      "zero-length events still take a slot (longer first)",
      [span("a", 60, 60), span("b", 60, 90)],
      { a: [1, 2], b: [0, 2] },
    ],
  ])("%s", (_name, items, want) => {
    const got = layoutDay(items);
    expect(Object.fromEntries([...got].map(([id, p]) => [id, [p.column, p.columns]]))).toEqual(
      want,
    );
  });
});

// The week of Monday 2026-03-02.
const WEEK = [
  "2026-03-02",
  "2026-03-03",
  "2026-03-04",
  "2026-03-05",
  "2026-03-06",
  "2026-03-07",
  "2026-03-08",
];
const days = (id: string, start: string, end: string) => ({ id, start, end });

describe("weekSegments", () => {
  it("places a multi-day item and lanes the overlapping ones", () => {
    const got = weekSegments(
      [
        days("conf", "2026-03-03", "2026-03-05"),
        days("call", "2026-03-04", "2026-03-04"),
        days("fri", "2026-03-06", "2026-03-06"),
      ],
      WEEK,
    );
    expect(got).toEqual([
      { id: "conf", column: 1, span: 3, lane: 0, continuesBefore: false, continuesAfter: false },
      { id: "call", column: 2, span: 1, lane: 1, continuesBefore: false, continuesAfter: false },
      { id: "fri", column: 4, span: 1, lane: 0, continuesBefore: false, continuesAfter: false },
    ]);
  });

  it("cuts items at the week's edges", () => {
    expect(weekSegments([days("trip", "2026-02-27", "2026-03-10")], WEEK)).toEqual([
      { id: "trip", column: 0, span: 7, lane: 0, continuesBefore: true, continuesAfter: true },
    ]);
    expect(weekSegments([days("late", "2026-03-08", "2026-03-12")], WEEK)).toEqual([
      { id: "late", column: 6, span: 1, lane: 0, continuesBefore: false, continuesAfter: true },
    ]);
    expect(weekSegments([days("other", "2026-03-10", "2026-03-11")], WEEK)).toEqual([]);
  });

  it("puts longer items first so they keep the top lanes", () => {
    const got = weekSegments(
      [days("short", "2026-03-02", "2026-03-02"), days("long", "2026-03-02", "2026-03-04")],
      WEEK,
    );
    expect(got.map((s) => [s.id, s.lane])).toEqual([
      ["long", 0],
      ["short", 1],
    ]);
  });
});

describe("hiddenPerDay", () => {
  it("counts what does not fit in the visible lanes per day", () => {
    const segments = weekSegments(
      [
        days("a", "2026-03-02", "2026-03-03"),
        days("b", "2026-03-02", "2026-03-02"),
        days("c", "2026-03-02", "2026-03-03"),
        days("d", "2026-03-03", "2026-03-03"),
      ],
      WEEK,
    );
    // Two lanes shown: Monday has a, c, b; Tuesday has a, c, d.
    expect(hiddenPerDay(segments, 2)).toEqual([1, 1, 0, 0, 0, 0, 0]);
    expect(hiddenPerDay(segments, 3)).toEqual([0, 0, 0, 0, 0, 0, 0]);
  });
});
