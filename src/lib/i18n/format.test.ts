import { describe, expect, it } from "vitest";
import { formatDate, formatDuration, formatMoney } from "./format";

// Intl uses (narrow) no-break spaces; compare with plain spaces.
const plain = (s: string) => s.replace(/\s/g, " ");

describe("formatMoney", () => {
  it.each([
    [123456, "RUB", "ru", "1 234,56 ₽"],
    [123456, "USD", "en", "$1,234.56"],
    [-5000, "EUR", "en", "-€50.00"],
    [1500, "JPY", "en", "¥1,500"],
    [1234, "KWD", "en", "KWD 1.234"],
    [0, "GBP", "ru", "0,00 £"],
  ])("%d %s in %s -> %s", (amount, currency, locale, expected) => {
    expect(plain(formatMoney(amount, currency, locale))).toBe(expected);
  });
});

describe("formatDate", () => {
  const ts = Date.UTC(2026, 2, 5, 14, 7); // 2026-03-05 14:07 UTC

  it("formats dates per locale in the given time zone", () => {
    expect(formatDate(ts, "en", { timeZone: "UTC" })).toBe("Mar 5, 2026");
    expect(formatDate(ts, "ru", { timeZone: "UTC" })).toBe("5 мар. 2026 г.");
  });

  it("can include time", () => {
    expect(plain(formatDate(ts, "ru", { timeZone: "Europe/Moscow", withTime: true }))).toBe(
      "5 мар. 2026 г., 17:07",
    );
  });
});

describe("formatDuration", () => {
  it.each([
    [0, "0:00"],
    [59, "0:00"],
    [60, "0:01"],
    [3_900, "1:05"],
    [36_000 * 10, "100:00"],
  ])("%d seconds -> %s", (seconds, expected) => {
    expect(formatDuration(seconds)).toBe(expected);
  });
});
