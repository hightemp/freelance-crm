import { $, browser, expect } from "@wdio/globals";
import { heading } from "../helpers";

describe("freelance-crm", () => {
  it("starts on the home screen with the main navigation", async () => {
    await expect(heading("Home")).toBeDisplayed();
    await expect($('nav[aria-label="Main"]')).toBeDisplayed();
  });

  it("is ready within the startup budget", async () => {
    // NFR-1: 2 s. The webview's clock starts with its navigation, close to
    // the process start; a debug build is slower than a release one.
    const ready = await browser.execute(
      () => performance.getEntriesByName("app-ready")[0]?.startTime ?? Infinity,
    );
    expect(ready).toBeLessThan(2000);
  });

  it("navigates to settings and persists a theme change", async () => {
    await $('a[href="/settings"]').click();
    await expect(heading("Settings")).toBeDisplayed();

    await $("aria/Dark").click();
    await expect($("html")).toHaveAttribute("data-theme", "dark");

    // Reload the UI: the theme must come back from the database.
    await browser.refresh();
    await expect($("html")).toHaveAttribute("data-theme", "dark");
  });
});
