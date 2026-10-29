import { $, browser, expect } from "@wdio/globals";
import { button, buttonByLabel, editor, field, menuItem, tab } from "../helpers";

describe("clients", () => {
  it("creates a company with a contact and a note, finds and archives it", async () => {
    await $('a[href="/clients"]').click();
    await expect($("h1")).toHaveText("Clients");

    // Create a company.
    await button("New client").click();
    await (await field("Short name")).setValue("Ромашка");
    await (await field("Legal name")).setValue("ООО «Ромашка»");
    await button("Save").click();
    await expect($("h1")).toHaveText("Ромашка");

    // Add a contact person.
    await tab("Contacts").click();
    await button("Add contact").click();
    const dialog = $('[role="dialog"]');
    await (await field("Last name")).setValue("Петрова");
    await (await field("First name")).setValue("Анна");
    await dialog.$("//button[normalize-space(.)='Save']").click();
    await expect($("//article//h3")).toHaveText("Петрова Анна");

    // Add a note.
    await tab("Notes").click();
    await (await editor("New note")).click();
    await browser.keys("Созвон в среду");
    await button("Add note").click();
    await expect($('[data-testid="note-body"]')).toHaveText("Созвон в среду");

    // Find it in the list.
    await $('a[href="/clients"]').click();
    await (await field("Search clients")).setValue("ромаш");
    const link = $("//table//a[normalize-space(.)='Ромашка']");
    await expect(link).toBeDisplayed();

    // Archive it from the card: it leaves the active list.
    await link.click();
    await buttonByLabel("More actions").click();
    await menuItem("Archive").click();
    await expect($("//*[normalize-space(.)='Archived']")).toBeDisplayed();
    await $('a[href="/clients"]').click();
    await expect($("//*[normalize-space(.)='No clients yet']")).toBeDisplayed();
  });
});
