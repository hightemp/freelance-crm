import { afterEach, describe, expect, it } from "vitest";
import { AppError } from "$lib/api/client";
import { errorMessage } from "./errors";
import { setAppLocale } from "./locale.svelte";

afterEach(() => setAppLocale("en"));

describe("errorMessage", () => {
  it("translates a known code with params", () => {
    const err = new AppError("money.currency_mismatch", { left: "RUB", right: "USD" });
    expect(errorMessage(err)).toBe("Cannot combine amounts in RUB and USD");
    setAppLocale("ru");
    expect(errorMessage(err)).toBe("Нельзя сложить суммы в RUB и USD");
  });

  it("falls back for unknown codes and shows the code", () => {
    expect(errorMessage(new AppError("something.weird"))).toBe(
      "Something went wrong (something.weird)",
    );
  });

  it("handles non-AppError values", () => {
    expect(errorMessage(new Error("boom"))).toBe("Something went wrong (internal.unknown)");
  });
});
