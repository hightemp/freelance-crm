import { describe, expect, it } from "vitest";
import { computeTotals, scaled } from "./invoice";

describe("scaled", () => {
  it("rounds half to even like the core", () => {
    expect(scaled(5, 1, 10)).toBe(0); // 0.5 → 0
    expect(scaled(15, 1, 10)).toBe(2); // 1.5 → 2
    expect(scaled(25, 1, 10)).toBe(2); // 2.5 → 2
    expect(scaled(26, 1, 10)).toBe(3);
    expect(scaled(-15, 1, 10)).toBe(-2);
  });
});

describe("computeTotals", () => {
  const item = (price: number, quantityMilli = 1000) => ({ price, quantityMilli });

  it("sums lines, applies a discount, then tax, and subtracts payments", () => {
    const t = computeTotals(
      [item(10000, 1500), item(333, 3000)],
      { type: "percent", value: 1000 },
      2000,
      5000,
    );
    expect(t.lines).toEqual([15000, 999]);
    expect(t.subtotal).toBe(15999);
    expect(t.discount).toBe(1600); // 10 % of 159.99 = 15.999 → 16.00
    expect(t.tax).toBe(2880); // 20 % of 143.99 = 28.798 → 28.80
    expect(t.total).toBe(17279);
    expect(t.due).toBe(12279);
  });

  it("caps a fixed discount at the subtotal", () => {
    const t = computeTotals([item(1000)], { type: "fixed", value: 5000 }, 0, 0);
    expect(t.discount).toBe(1000);
    expect(t.total).toBe(0);
  });

  it("an empty invoice is all zeros", () => {
    expect(computeTotals([], { type: "none" }, 2000, 0)).toEqual({
      lines: [],
      subtotal: 0,
      discount: 0,
      tax: 0,
      total: 0,
      paid: 0,
      due: 0,
    });
  });
});
