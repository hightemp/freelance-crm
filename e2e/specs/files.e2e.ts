import { writeFileSync } from "node:fs";
import { join } from "node:path";
import { $, browser, expect } from "@wdio/globals";
import { button, field, tab } from "../helpers";

// A 1×1 PNG.
const PNG = Buffer.from(
  "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==",
  "base64",
);

/** Whether an <img> has actually loaded (the crmfile:// protocol works). */
const loaded = (selector: string) =>
  browser.execute((s) => {
    const img = document.querySelector<HTMLImageElement>(s);
    return !!img && img.complete && img.naturalWidth > 0;
  }, selector);

describe("files", () => {
  it("attaches an image and shows it in the gallery and the lightbox", async () => {
    await $('a[href="/clients"]').click();
    await button("New client").click();
    await (await field("Short name")).setValue("Файлы");
    await button("Save").click();
    await expect($("h1")).toHaveText("Файлы");

    // The file dialog and drag & drop are native; attach through the same
    // command they call.
    const path = join(process.env.FREELANCE_CRM_DATA_DIR!, "dot.png");
    writeFileSync(path, PNG);
    const clientId = (await browser.getUrl()).split("/").pop()!;
    await browser.execute(
      (id, p) =>
        (
          window as unknown as {
            __TAURI_INTERNALS__: { invoke: (c: string, a: unknown) => Promise<unknown> };
          }
        ).__TAURI_INTERNALS__.invoke("attach_file", {
          owner: { type: "client", id },
          path: p,
          encrypt: false,
        }),
      clientId,
      path,
    );

    await tab("Files").click();
    await expect($("//li[contains(., 'dot.png')]")).toBeDisplayed();
    await button("Gallery").click();
    await browser.waitUntil(() => loaded('img[alt="dot.png"]'), {
      timeoutMsg: "thumbnail did not load",
    });
    await $('button[aria-label="View dot.png"]').click();
    await browser.waitUntil(() => loaded('[role="dialog"] img[alt="dot.png"]'), {
      timeoutMsg: "full image did not load",
    });
    if (process.env.E2E_SCREENSHOTS) await browser.saveScreenshot(process.env.E2E_SCREENSHOTS);
  });
});
