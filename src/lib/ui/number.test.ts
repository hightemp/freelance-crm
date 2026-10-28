import { describe, expect, it } from "vitest";
import { formatMinor, formatNumber, parseMinor, parseNumber } from "./number";

describe("parseNumber", () => {
  it.each([
    ["1234", "en", 1234],
    ["1,234.5", "en", 1234.5],
    ["1 234,5", "ru", 1234.5],
    ["1 234,5", "ru", 1234.5],
    ["1.5", "ru", 1.5], // dot is accepted as decimal separator in ru too
    ["-12,5", "ru", -12.5],
    ["  42 ", "en", 42],
    ["", "en", null],
    ["abc", "en", null],
    ["1,2,3.4.5", "en", null],
    ["1.2.3", "ru", null],
  ])("%j in %s -> %s", (input, locale, expected) => {
    expect(parseNumber(input, locale)).toBe(expected);
  });
});

describe("parseMinor", () => {
  it.each([
    ["1 234,56", "ru", 2, 123456],
    ["1,234.56", "en", 2, 123456],
    ["1234", "en", 2, 123400],
    ["0,1", "ru", 2, 10],
    ["0.07", "en", 2, 7],
    ["-5.5", "en", 2, -550],
    ["1500", "en", 0, 1500],
    ["1.234", "en", 3, 1234],
    ["90071992547409.91", "en", 2, 9007199254740991],
  ])("%j in %s with %d digits -> %d", (input, locale, digits, expected) => {
    expect(parseMinor(input, locale, digits)).toBe(expected);
  });

  it("does not use floating point (0.1 + 0.2 style inputs stay exact)", () => {
    expect(parseMinor("0.29", "en", 2)).toBe(29);
    expect(parseMinor("1.15", "en", 2)).toBe(115);
    expect(parseMinor("4.35", "en", 2)).toBe(435);
  });

  it.each([
    ["1.234", "en", 2], // too many fraction digits
    ["1.5", "en", 0],
    ["", "en", 2],
    ["12a", "en", 2],
    ["99999999999999999", "en", 2], // beyond safe integer
  ])("%j in %s with %d digits -> null", (input, locale, digits) => {
    expect(parseMinor(input, locale, digits)).toBeNull();
  });
});

describe("formatting for inputs", () => {
  it("formats numbers without currency symbols", () => {
    expect(formatNumber(1234.5, "ru").replace(/\s/g, " ")).toBe("1 234,5");
    expect(formatNumber(1234.5, "en")).toBe("1,234.5");
  });

  it("formats minor units with exact fraction digits", () => {
    expect(formatMinor(123456, "ru", 2).replace(/\s/g, " ")).toBe("1 234,56");
    expect(formatMinor(-7, "en", 2)).toBe("-0.07");
    expect(formatMinor(1500, "en", 0)).toBe("1,500");
  });

  it("round-trips formatMinor -> parseMinor", () => {
    for (const value of [0, 1, 99, 100, 123456, -98765, 9007199254740991]) {
      for (const locale of ["en", "ru"]) {
        expect(parseMinor(formatMinor(value, locale, 2), locale, 2)).toBe(value);
      }
    }
  });
});
