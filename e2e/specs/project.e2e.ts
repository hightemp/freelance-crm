import { $, browser, expect } from "@wdio/globals";
import { button, field, heading, tab } from "../helpers";

describe("projects", () => {
  it("creates a project from the client card and opens its card", async () => {
    await $('a[href="/clients"]').click();
    await button("New client").click();
    await (await field("Short name")).setValue("Вектор");
    await (await field("Hourly rate")).setValue("2500");
    await button("Save").click();
    await expect(heading("Вектор")).toBeDisplayed();

    await tab("Projects").click();
    await button("New project").click();
    await expect(heading("New project")).toBeDisplayed();
    await (await field("Name")).setValue("Поддержка сайта");
    await $(`//*[@role="radio" and normalize-space(.)="Hourly"]`).click();
    await button("Save").click();

    // The client's rate is inherited.
    await expect(heading("Поддержка сайта")).toBeDisplayed();
    await expect($("//a[normalize-space(.)='Вектор']")).toBeDisplayed();
    await expect($("//dd[contains(., '2,500.00')]")).toBeDisplayed();
    if (process.env.E2E_SCREENSHOTS) await browser.saveScreenshot(process.env.E2E_SCREENSHOTS);

    await $('a[href="/projects"]').click();
    await expect($("//table//a[normalize-space(.)='Поддержка сайта']")).toBeDisplayed();
  });
});
