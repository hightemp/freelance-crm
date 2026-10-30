import { $, browser, expect } from "@wdio/globals";
import { button, field, heading, option, select, tab } from "../helpers";

/** Drags with the mouse in small steps, as svelte-dnd-action expects. */
async function drag(source: WebdriverIO.Element, target: WebdriverIO.Element) {
  const from = await source.getLocation();
  const fromSize = await source.getSize();
  const to = await target.getLocation();
  const toSize = await target.getSize();
  const start = { x: Math.round(from.x + fromSize.width / 2), y: Math.round(from.y + 10) };
  const end = { x: Math.round(to.x + toSize.width / 2), y: Math.round(to.y + toSize.height / 2) };
  const steps = 10;
  const moves = Array.from({ length: steps }, (_, i) => ({
    type: "pointerMove" as const,
    duration: 30,
    x: Math.round(start.x + ((end.x - start.x) * (i + 1)) / steps),
    y: Math.round(start.y + ((end.y - start.y) * (i + 1)) / steps),
  }));
  await browser.performActions([
    {
      type: "pointer",
      id: "mouse",
      parameters: { pointerType: "mouse" },
      actions: [
        { type: "pointerMove", duration: 0, x: start.x, y: start.y },
        { type: "pointerDown", button: 0 },
        { type: "pause", duration: 100 },
        ...moves,
        { type: "pause", duration: 200 },
        { type: "pointerUp", button: 0 },
      ],
    },
  ]);
  await browser.releaseActions();
}

describe("tasks", () => {
  it("creates a task in a project, moves it on the board and closes it", async () => {
    // A client with a project.
    await $('a[href="/clients"]').click();
    await button("New client").click();
    await (await field("Short name")).setValue("Альфа");
    await button("Save").click();
    await expect(heading("Альфа")).toBeDisplayed();
    await tab("Projects").click();
    await button("New project").click();
    await (await field("Name")).setValue("Портал");
    await (await field("Budget")).setValue("1000");
    await button("Save").click();
    await expect(heading("Портал")).toBeDisplayed();

    // A task in the project, with a checklist.
    await tab("Tasks").click();
    await button("New task").click();
    await expect(select("Project")).toHaveText("Портал");
    await (await field("Title")).setValue("Макет главной");
    await button("Save").click();
    await expect(heading("Макет главной")).toBeDisplayed();
    await (await field("New item")).setValue("Шапка");
    await button("Add item").click();
    await expect($("//li[normalize-space(.)='Шапка']")).toBeDisplayed();

    // Drag it on the board from "To do" to "In progress".
    await $('a[href="/tasks"]').click();
    await tab("Board").click();
    const card = $(`//*[@role="listitem" and @aria-label="Макет главной"]`);
    const inProgress = $(`//*[@role="list" and @aria-label="Tasks in In progress"]`);
    await drag(card, inProgress);
    await expect(inProgress.$(`.//*[@aria-label="Макет главной"]`)).toBeExisting();
    await browser.refresh();
    await tab("Board").click();
    await expect(
      $(`//*[@aria-label="Tasks in In progress"]//*[@aria-label="Макет главной"]`),
    ).toBeExisting();

    // Check the item off and close the task.
    await $("//a[normalize-space(.)='Макет главной']").click();
    await expect(select("Column")).toHaveText("In progress");
    await $(`//li//*[@role="checkbox"]`).click();
    await expect($("//*[normalize-space(.)='100% done']")).toBeDisplayed();
    await select("Column").click();
    await option("Done").click();
    await expect($("//*[normalize-space(.)='Completed']")).toBeDisplayed();
  });
});
