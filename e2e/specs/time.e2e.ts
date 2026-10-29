import { $, browser, expect } from "@wdio/globals";
import { button, field } from "../helpers";

describe("time tracking", () => {
  it("runs a timer from the header and shows it in the journal", async () => {
    await $('a[href="/time"]').click();
    await expect($("h1")).toHaveText("Time");

    await button("Start timer").click();
    await (await field("What are you working on?")).setValue("Созвон с клиентом");
    await $('[role="dialog"]').$(`.//button[normalize-space(.)="Start"]`).click();
    const clock = $('[role="timer"]');
    await expect(clock).toBeDisplayed();
    await browser.waitUntil(async () => (await clock.getText()) !== "0:00:00", {
      timeoutMsg: "the timer does not tick",
    });
    await browser.pause(1000);
    await $('button[aria-label="Stop timer"]').click();
    await expect(button("Start timer")).toBeDisplayed();

    await expect($("//li[contains(., 'Созвон с клиентом')]")).toBeDisplayed();
    if (process.env.E2E_SCREENSHOTS) await browser.saveScreenshot(process.env.E2E_SCREENSHOTS);
  });
});
