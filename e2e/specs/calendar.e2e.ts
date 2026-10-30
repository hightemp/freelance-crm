import { join } from "node:path";
import { $, $$, browser, expect } from "@wdio/globals";
import { button, field, heading, option, select } from "../helpers";

const startsWith = (role: string, text: string) =>
  $(
    `//*[@role=${JSON.stringify(role)} and starts-with(normalize-space(.), ${JSON.stringify(text)})]`,
  );

/** Texts of the agenda's items, in order. */
const agenda = async () =>
  (await $$("//section//li//button").map((b) => b.getText())).map((t) => t.replace(/\s+/g, " "));

/** Calls a backend command the way the UI does (native file dialogs cannot be driven). */
const invoke = (cmd: string, args: Record<string, unknown>) =>
  browser.execute(
    (c, a) =>
      (
        window as unknown as {
          __TAURI_INTERNALS__: { invoke: (c: string, a: unknown) => Promise<unknown> };
        }
      ).__TAURI_INTERNALS__.invoke(c, a),
    cmd,
    args,
  );

async function scope(choice: string) {
  const dialog = $('[role="dialog"]');
  await dialog.$(`//*[@role="radio" and normalize-space(.)=${JSON.stringify(choice)}]`).click();
  await dialog.$(`.//button[normalize-space(.)="OK"]`).click();
}

describe("calendar", () => {
  it("edits one occurrence of a weekly event and survives an ICS round trip", async () => {
    await $('a[href="/calendar"]').click();
    await expect(heading("Calendar")).toBeDisplayed();

    // A weekly event, four times from today.
    await button("New event").click();
    await (await field("Title")).setValue("Еженедельная планёрка");
    await select("Repeat").click();
    await startsWith("option", "Every week on").click();
    await select("Ends").click();
    await option("After a number of times").click();
    await (await field("Times")).setValue("4");
    await $('[role="dialog"]').$(`.//button[normalize-space(.)="Save"]`).click();

    await button("Agenda").click();
    await browser.waitUntil(async () => (await agenda()).length === 4, {
      timeoutMsg: "four occurrences expected",
    });

    // Change only the second one.
    await $$("//section//li//button")[1].click();
    const title = await field("Title");
    await title.clearValue();
    await title.setValue("Планёрка (перенос)");
    await $('[role="dialog"]').$(`.//button[normalize-space(.)="Save"]`).click();
    await scope("This event");
    await browser.waitUntil(async () => (await agenda()).some((t) => t.includes("перенос")));
    const before = await agenda();
    expect(before.filter((t) => t.includes("Еженедельная планёрка"))).toHaveLength(3);
    if (process.env.E2E_SCREENSHOTS) await browser.saveScreenshot(process.env.E2E_SCREENSHOTS);

    // Export, delete the whole series, import the file back.
    const path = join(process.env.FREELANCE_CRM_DATA_DIR!, "calendar.ics");
    await invoke("export_ics", {
      path,
      filter: { clientId: null, projectId: null, from: null, to: null },
    });
    await $$("//section//li//button")[0].click();
    await $('[role="dialog"]').$(`.//button[normalize-space(.)="Delete event"]`).click();
    await scope("All events");
    await expect($("//*[normalize-space(.)='Nothing planned']")).toBeDisplayed();

    await invoke("import_ics", { path, uids: null, link: { clientId: null, projectId: null } });
    await button("Week").click();
    await button("Agenda").click();
    await browser.waitUntil(async () => (await agenda()).length === 4, {
      timeoutMsg: "the imported occurrences did not appear",
    });
    expect(await agenda()).toEqual(before);

    // Snapshots of the other views for visual review.
    const shots = process.env.E2E_SCREENSHOTS;
    if (shots) {
      for (const view of ["Month", "Week"]) {
        await button(view).click();
        await browser.pause(300);
        await browser.saveScreenshot(shots.replace(/\.png$/, `-${view.toLowerCase()}.png`));
      }
    }
  });
});
