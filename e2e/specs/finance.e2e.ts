import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { $, browser, expect } from "@wdio/globals";
import { button, buttonByLabel, field, menuItem, option, select, tab } from "../helpers";

/** Calls a backend command the way the UI does (native file dialogs cannot be driven). */
const invoke = <T>(cmd: string, args: Record<string, unknown>) =>
  browser.execute(
    (c, a) =>
      (
        window as unknown as {
          __TAURI_INTERNALS__: { invoke: (c: string, a: unknown) => Promise<unknown> };
        }
      ).__TAURI_INTERNALS__.invoke(c, a),
    cmd,
    args,
  ) as Promise<T>;

const HOUR = 3_600_000;

describe("finance", () => {
  it("bills tracked time, saves the PDF, takes a partial payment and shows it", async () => {
    // A client billed 1 500 ₽ an hour with two hours of work done today.
    const client = await invoke<{ id: string }>("create_client", {
      input: {
        kind: "company",
        lastName: null,
        firstName: null,
        middleName: null,
        companyName: "Счета и Ко",
        legalName: null,
        status: "active",
        defaultCurrency: "RUB",
        defaultRateMinor: 150000,
      },
    });
    const end = Date.now() - 60_000;
    await invoke("create_time_entry", {
      input: {
        target: { taskId: null, projectId: null, clientId: client.id },
        description: "Вёрстка",
        startedAt: end - 2 * HOUR,
        endedAt: end,
        billable: true,
      },
    });

    await $('a[href="/finance"]').click();
    await expect($("h1")).toHaveText("Finance");
    await tab("Invoices").click();

    // The "from time" wizard shows the entry and its total.
    await button("From time").click();
    // A required select: its label ends with " *".
    await $(
      `//span[starts-with(normalize-space(.), "Client")]/following-sibling::button[@role="combobox"]`,
    ).click();
    await option("Счета и Ко").click();
    await expect($("//*[normalize-space(.)='Total: 2:00']")).toBeDisplayed();
    await button("Create draft").click();

    // The draft opens in the editor: 2 h × 1 500 ₽; send it. (Intl puts a
    // no-break space after the currency code.)
    const editor = $('[role="dialog"]');
    await expect(editor.$('[data-testid="invoice-total"]')).toHaveText(/RUB\s3,000\.00/);
    const number = await (await field("Number")).getValue();
    await (await select("Status")).click();
    await option("Sent").click();
    await editor.$(`.//button[normalize-space(.)="Save"]`).click();
    await expect($(`//tr[contains(., ${JSON.stringify(number)})]`)).toBeDisplayed();

    // Saving the PDF goes through the native dialog: call its command.
    const invoices = await invoke<{ id: string; number: string }[]>("list_invoices", {
      clientId: client.id,
      projectId: null,
    });
    const pdf = join(process.env.FREELANCE_CRM_DATA_DIR!, `${number}.pdf`);
    await invoke("save_invoice_pdf", { id: invoices[0].id, path: pdf });
    expect(existsSync(pdf)).toBe(true);
    expect(readFileSync(pdf).subarray(0, 5).toString()).toBe("%PDF-");

    // A partial payment.
    await buttonByLabel(`Actions for invoice ${number}`).click();
    await menuItem("Record payment").click();
    const payment = $('[role="dialog"]');
    const amount = await field("Amount");
    await amount.clearValue();
    await amount.setValue("1000");
    await payment.$(`.//button[normalize-space(.)="Save"]`).click();
    const row = $(`//tr[contains(., ${JSON.stringify(number)})]`);
    await expect(row).toHaveText(/Partially paid/);

    // The overview counts the payment as income and the rest as unpaid.
    await tab("Overview").click();
    // The default currency (USD here) comes first: switch to roubles.
    await $('//*[@role="radio" and normalize-space(.)="RUB"]').click();
    await expect($('[role="group"][aria-label="Income"]')).toHaveText(/RUB\s1,000\.00/);
    await expect($('[role="group"][aria-label="Unpaid"]')).toHaveText(/RUB\s2,000\.00/);
    if (process.env.E2E_SCREENSHOTS) await browser.saveScreenshot(process.env.E2E_SCREENSHOTS);
  });
});
