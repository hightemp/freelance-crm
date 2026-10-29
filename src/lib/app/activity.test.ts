import { fireEvent } from "@testing-library/svelte";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { installActivityReporter } from "./activity";

let uninstall: () => void = () => {};

beforeEach(() => vi.useFakeTimers());
afterEach(() => {
  uninstall();
  vi.useRealTimers();
});

describe("installActivityReporter", () => {
  it("reports input activity at most once per interval", async () => {
    const report = vi.fn();
    uninstall = installActivityReporter(report, 30_000);
    await fireEvent.keyDown(window, { key: "a" });
    await fireEvent.pointerDown(window);
    expect(report).toHaveBeenCalledTimes(1);

    vi.advanceTimersByTime(29_999);
    await fireEvent.wheel(window);
    expect(report).toHaveBeenCalledTimes(1);
    vi.advanceTimersByTime(1);
    await fireEvent.wheel(window);
    expect(report).toHaveBeenCalledTimes(2);
  });

  it("stops after uninstalling", async () => {
    const report = vi.fn();
    installActivityReporter(report, 30_000)();
    await fireEvent.keyDown(window, { key: "a" });
    expect(report).not.toHaveBeenCalled();
  });
});
