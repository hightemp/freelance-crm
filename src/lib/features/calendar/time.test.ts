import { describe, expect, it } from "vitest";
import { itemDays, localDate, minutesOfDay, viewRange } from "./time";

// Vitest runs with TZ=UTC (vite.config.ts); ranges are in local time.

describe("viewRange", () => {
  it("covers six weeks for a month", () => {
    const r = viewRange("month", "2026-03-18", 1);
    expect(r.days[0]).toBe("2026-02-23");
    expect(r.days).toHaveLength(42);
    expect(r.title).toBe("2026-03");
  });

  it("covers the week by the week start", () => {
    expect(viewRange("week", "2026-03-18", 1).days).toEqual([
      "2026-03-16",
      "2026-03-17",
      "2026-03-18",
      "2026-03-19",
      "2026-03-20",
      "2026-03-21",
      "2026-03-22",
    ]);
    expect(viewRange("week", "2026-03-18", 0).days[0]).toBe("2026-03-15");
  });

  it("covers a day and thirty days of agenda", () => {
    expect(viewRange("day", "2026-03-18", 1).days).toEqual(["2026-03-18"]);
    const agenda = viewRange("agenda", "2026-03-18", 1);
    expect(agenda.days).toHaveLength(30);
    expect(agenda.days[29]).toBe("2026-04-16");
  });

  it("turns the days into a local half-open range", () => {
    const r = viewRange("day", "2026-03-18", 1);
    expect(r.to - r.from).toBe(24 * 3600_000);
    expect(new Date(r.from).getHours()).toBe(0);
    expect(localDate(r.from)).toBe("2026-03-18");
  });

  it("steps by its own unit", () => {
    expect(viewRange("month", "2026-01-31", 1).next).toBe("2026-02-28");
    expect(viewRange("week", "2026-03-18", 1).previous).toBe("2026-03-11");
    expect(viewRange("day", "2026-03-18", 1).next).toBe("2026-03-19");
  });
});

describe("item days", () => {
  const timed = (startAt: number, endAt: number) => ({
    allDay: false,
    start: "",
    end: "",
    startAt,
    endAt,
  });

  it("uses the dates of all-day items as they are", () => {
    expect(
      itemDays({ allDay: true, start: "2026-05-01", end: "2026-05-03", startAt: 0, endAt: 0 }),
    ).toEqual({
      start: "2026-05-01",
      end: "2026-05-03",
    });
  });

  it("spans the local days of timed items, ending before midnight", () => {
    const at = (s: string) => Date.parse(s);
    expect(itemDays(timed(at("2026-03-18T22:00:00Z"), at("2026-03-19T00:00:00Z")))).toEqual({
      start: "2026-03-18",
      end: "2026-03-18",
    });
    expect(itemDays(timed(at("2026-03-18T22:00:00Z"), at("2026-03-20T01:00:00Z")))).toEqual({
      start: "2026-03-18",
      end: "2026-03-20",
    });
    expect(minutesOfDay(at("2026-03-18T09:30:00Z"))).toBe(570);
  });
});
