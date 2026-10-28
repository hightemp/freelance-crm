import { $, browser, expect } from "@wdio/globals";

describe("freelance-crm", () => {
  it("starts on the home screen with the main navigation", async () => {
    await expect($("h1")).toHaveText("Home");
    await expect($('nav[aria-label="Main"]')).toBeDisplayed();
  });

  it("navigates to settings and persists a theme change", async () => {
    await $('a[href="/settings"]').click();
    await expect($("h1")).toHaveText("Settings");

    await $("aria/Dark").click();
    await expect($("html")).toHaveAttribute("data-theme", "dark");

    // Reload the UI: the theme must come back from the database.
    await browser.refresh();
    await expect($("html")).toHaveAttribute("data-theme", "dark");
  });
});
