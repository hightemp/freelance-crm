import { createRawSnippet, type Snippet } from "svelte";

/** A snippet rendering plain text, for passing `children` in tests. */
export function text(value: string): Snippet {
  return createRawSnippet(() => ({ render: () => `<span>${value}</span>` }));
}

/** A snippet with one reactive argument, rendering `prefix:arg`. */
export function textWithArg(prefix: string): Snippet<[string]> {
  return createRawSnippet((arg: () => string) => ({
    render: () => "<span></span>",
    setup: (node) => {
      $effect(() => {
        node.textContent = `${prefix}:${arg()}`;
      });
    },
  }));
}
