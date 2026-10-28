import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { initTheme, setTheme, theme } from "./theme.svelte";

let listeners: ((e: { matches: boolean }) => void)[] = [];
let systemDark = false;

beforeEach(() => {
  listeners = [];
  systemDark = false;
  vi.stubGlobal(
    "matchMedia",
    vi.fn(() => ({
      get matches() {
        return systemDark;
      },
      addEventListener: (_: string, fn: (e: { matches: boolean }) => void) => listeners.push(fn),
      removeEventListener: vi.fn(),
    })),
  );
});

afterEach(() => {
  setTheme("system");
  vi.unstubAllGlobals();
});

const applied = () => document.documentElement.dataset.theme;

describe("theme", () => {
  it("applies explicit light and dark themes", () => {
    initTheme();
    setTheme("dark");
    expect(theme.mode).toBe("dark");
    expect(applied()).toBe("dark");
    setTheme("light");
    expect(applied()).toBe("light");
  });

  it("follows the OS in system mode, including live changes", () => {
    systemDark = true;
    initTheme();
    setTheme("system");
    expect(applied()).toBe("dark");
    systemDark = false;
    listeners.forEach((fn) => fn({ matches: false }));
    expect(applied()).toBe("light");
  });

  it("ignores OS changes when a theme is chosen explicitly", () => {
    initTheme();
    setTheme("light");
    systemDark = true;
    listeners.forEach((fn) => fn({ matches: true }));
    expect(applied()).toBe("light");
  });
});
