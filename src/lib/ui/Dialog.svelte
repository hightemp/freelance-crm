<script lang="ts">
  import type { Snippet } from "svelte";
  import { m } from "$lib/paraglide/messages";
  import { portal, trapFocus } from "./overlay";

  let {
    open = $bindable(false),
    title,
    description,
    role = "dialog",
    dismissable = true,
    onclose,
    children,
    footer,
  }: {
    open?: boolean;
    title: string;
    description?: string;
    role?: "dialog" | "alertdialog";
    /** Close on Escape, backdrop click and the close button. */
    dismissable?: boolean;
    /** Called when the dialog closes itself (Escape, backdrop, close button). */
    onclose?: () => void;
    children?: Snippet;
    footer?: Snippet;
  } = $props();

  const id = $props.id();

  function dismiss() {
    if (!dismissable) return;
    open = false;
    onclose?.();
  }

  function onkeydown(event: KeyboardEvent) {
    if (event.key === "Escape") {
      event.stopPropagation();
      dismiss();
    }
  }
</script>

{#if open}
  <div {@attach portal} class="fixed inset-0 z-50 grid place-items-center p-4">
    <div
      data-testid="dialog-backdrop"
      class="absolute inset-0 bg-black/40"
      aria-hidden="true"
      onclick={dismiss}
    ></div>
    <div
      {@attach trapFocus}
      {role}
      aria-modal="true"
      aria-labelledby="{id}-title"
      aria-describedby={description ? `${id}-description` : undefined}
      tabindex="-1"
      {onkeydown}
      class="relative flex max-h-full w-full max-w-md flex-col gap-3 rounded-[var(--radius)]
        border border-border bg-bg p-5 shadow-xl"
    >
      <h2 id="{id}-title" class="pr-8 text-base font-semibold text-fg">{title}</h2>
      {#if description}
        <p id="{id}-description" class="text-sm text-muted">{description}</p>
      {/if}
      {#if children}<div class="overflow-auto">{@render children()}</div>{/if}
      {#if footer}<div class="flex justify-end gap-2 pt-2">{@render footer()}</div>{/if}
      {#if dismissable}
        <!-- Last in DOM so Tab starts at the content; placed top-right visually. -->
        <button
          type="button"
          aria-label={m.ui_close()}
          onclick={dismiss}
          class="absolute top-3 right-3 grid size-7 place-items-center rounded-[var(--radius)]
            text-muted hover:bg-surface-hover hover:text-fg"
        >
          <svg width="12" height="12" viewBox="0 0 12 12" aria-hidden="true">
            <path d="M2 2l8 8M10 2l-8 8" stroke="currentColor" stroke-width="1.5" />
          </svg>
        </button>
      {/if}
    </div>
  </div>
{/if}
