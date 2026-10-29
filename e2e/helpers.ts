// Selectors by visible text and labels, matching how users find controls.
// `aria/` selectors are avoided for fields: required labels end with " *".
// Keys sent by the embedded WebDriver are untrusted events, so Enter does
// not submit forms: click the submit button instead.
import { $ } from "@wdio/globals";

const quoted = (text: string) => JSON.stringify(text);

export const button = (text: string) => $(`//button[normalize-space(.)=${quoted(text)}]`);

export const buttonByLabel = (label: string) => $(`button[aria-label=${quoted(label)}]`);

export const tab = (text: string) => $(`//*[@role="tab" and normalize-space(.)=${quoted(text)}]`);

export const menuItem = (text: string) =>
  $(`//*[@role="menuitem" and normalize-space(.)=${quoted(text)}]`);

/** Text field or textarea by the start of its label. */
export async function field(label: string) {
  const id = await $(`//label[starts-with(normalize-space(.), ${quoted(label)})]`).getAttribute(
    "for",
  );
  return $(`[id=${quoted(id)}]`);
}

/** Markdown editor (CodeMirror) by its label. */
export async function editor(label: string) {
  const id = await $(`//span[normalize-space(.)=${quoted(label)}]`).getAttribute("id");
  return $(`[aria-labelledby=${quoted(id)}]`);
}
