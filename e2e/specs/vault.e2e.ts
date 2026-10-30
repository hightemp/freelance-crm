import { $, browser, expect } from "@wdio/globals";
import { button, field, heading, option, select } from "../helpers";

describe("vault", () => {
  it("creates a vault and an SSH secret, locks, unlocks and copies", async () => {
    await $('a[href="/vault"]').click();
    await (await field("Master password")).setValue("correct horse battery staple");
    await (await field("Repeat the master password")).setValue("correct horse battery staple");
    await button("Create vault").click();

    // The recovery key is shown once and must be confirmed.
    const key = $('[role="group"][aria-label="Recovery key"]');
    await expect(key).toBeDisplayed();
    const groups = (await key.getText()).split(/\s+/);
    await button("I have saved it").click();
    const hint = await $("//p[starts-with(normalize-space(.), 'Enter groups')]").getText();
    for (const n of hint.match(/\d+/g)!.map(Number)) {
      await (await field(`Group ${n}`)).setValue(groups[n - 1]);
    }
    await button("Confirm").click();

    // An SSH secret with a generated key pair.
    await button("New secret").click();
    await (await field("Title")).setValue("Deploy key");
    await select("Type").click();
    await option("SSH key").click();
    await button("Generate key pair").click();
    const dialog = $('[role="dialog"]');
    await (await field("Comment")).setValue("deploy@ci");
    await dialog.$(`.//button[normalize-space(.)="Generate key pair"]`).click();
    await expect(await field("Public key")).toHaveValue(expect.stringContaining("ssh-ed25519"));
    await button("Save").click();
    await expect(heading("Deploy key")).toBeDisplayed();

    // The private key is hidden until asked for.
    await expect($("//*[normalize-space(.)='Hidden']")).toBeDisplayed();
    await $('button[aria-label="Show Private key"]').click();
    await expect($("//dd[contains(., 'OPENSSH PRIVATE KEY')]")).toBeDisplayed();

    // Locking hides everything and asks for the master password.
    await $('button[aria-label="Vault unlocked. Lock now"]').click();
    await expect(button("Unlock")).toBeDisplayed();
    await expect($("//*[contains(., 'OPENSSH PRIVATE KEY')]")).not.toBeExisting();

    await (await field("Master password")).setValue("correct horse battery staple");
    await button("Unlock").click();
    await expect(heading("Deploy key")).toBeDisplayed();
    await $('button[aria-label="Copy Private key"]').click();
    await expect($("//*[normalize-space(.)='Private key copied']")).toBeDisplayed();
    if (process.env.E2E_SCREENSHOTS) await browser.saveScreenshot(process.env.E2E_SCREENSHOTS);
  });
});
