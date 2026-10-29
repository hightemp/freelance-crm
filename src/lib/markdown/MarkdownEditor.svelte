<script lang="ts">
  import { untrack } from "svelte";
  import { defaultKeymap, history, historyKeymap } from "@codemirror/commands";
  import { markdown } from "@codemirror/lang-markdown";
  import { EditorState } from "@codemirror/state";
  import { EditorView, keymap } from "@codemirror/view";
  import { m } from "$lib/paraglide/messages";
  import Markdown from "./Markdown.svelte";

  let {
    label,
    value = $bindable(""),
    rows = 4,
  }: {
    label: string;
    value?: string;
    /** Minimum visible height in lines. */
    rows?: number;
  } = $props();

  const id = $props.id();
  let mode = $state<"write" | "preview">("write");
  let view: EditorView | undefined;

  const theme = EditorView.theme({
    "&": {
      border: "1px solid var(--color-border)",
      borderRadius: "var(--radius)",
      backgroundColor: "var(--color-bg)",
      color: "var(--color-fg)",
      fontSize: "0.875rem",
    },
    "&.cm-focused": { outline: "2px solid var(--color-focus)", outlineOffset: "-1px" },
    ".cm-content": {
      fontFamily: "var(--font-sans)",
      padding: "6px 2px",
      caretColor: "var(--color-fg)",
    },
    ".cm-scroller": { fontFamily: "var(--font-sans)", lineHeight: "1.5" },
  });

  // The editor is created once; later `value` changes are applied below.
  function editor(node: HTMLElement) {
    view = new EditorView({
      parent: node,
      state: EditorState.create({
        doc: untrack(() => value),
        extensions: [
          history(),
          keymap.of([...defaultKeymap, ...historyKeymap]),
          markdown(),
          EditorView.lineWrapping,
          theme,
          EditorView.theme({ ".cm-content": { minHeight: `${rows * 1.5}em` } }),
          EditorView.contentAttributes.of({ "aria-labelledby": `${id}-label` }),
          EditorView.updateListener.of((update) => {
            if (update.docChanged) value = update.state.doc.toString();
          }),
        ],
      }),
    });
    return () => {
      view?.destroy();
      view = undefined;
    };
  }

  // Apply external changes, e.g. the form clearing the draft after saving.
  $effect(() => {
    const next = value;
    if (view && next !== view.state.doc.toString()) {
      view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: next } });
    }
  });

  const modeClass = (active: boolean) =>
    `rounded px-2 py-0.5 text-xs ${active ? "bg-surface-hover text-fg" : "text-muted hover:text-fg"}`;
</script>

<div class="flex flex-col gap-1" data-md-editor>
  <div class="flex items-center justify-between gap-2">
    <span id="{id}-label" class="text-sm font-medium text-fg">{label}</span>
    <div class="flex gap-1" role="group" aria-label={m.md_mode()}>
      <button
        type="button"
        class={modeClass(mode === "write")}
        aria-pressed={mode === "write"}
        onclick={() => (mode = "write")}>{m.md_write()}</button
      >
      <button
        type="button"
        class={modeClass(mode === "preview")}
        aria-pressed={mode === "preview"}
        onclick={() => (mode = "preview")}>{m.md_preview()}</button
      >
    </div>
  </div>
  <!-- Kept mounted while previewing so undo history and cursor survive. -->
  <div hidden={mode !== "write"} {@attach editor}></div>
  {#if mode === "preview"}
    <div
      class="rounded-[var(--radius)] border border-border px-3 py-2"
      style:min-height="{rows * 1.5 + 1}em"
    >
      {#if value.trim() === ""}
        <p class="text-sm text-muted">{m.md_preview_empty()}</p>
      {:else}
        <Markdown source={value} />
      {/if}
    </div>
  {/if}
</div>
