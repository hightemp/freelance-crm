<script lang="ts">
  import type { Snippet } from "svelte";

  interface Item {
    id: string;
    title: string;
  }

  let {
    items,
    content,
    open = $bindable([]),
    multiple = false,
  }: {
    items: Item[];
    content: Snippet<[string]>;
    open?: string[];
    multiple?: boolean;
  } = $props();

  const id = $props.id();

  function toggle(itemId: string) {
    if (open.includes(itemId)) open = open.filter((x) => x !== itemId);
    else open = multiple ? [...open, itemId] : [itemId];
  }
</script>

<div class="flex flex-col divide-y divide-border rounded-[var(--radius)] border border-border">
  {#each items as item (item.id)}
    {@const expanded = open.includes(item.id)}
    <div>
      <h3>
        <button
          id="{id}-header-{item.id}"
          type="button"
          aria-expanded={expanded}
          aria-controls="{id}-region-{item.id}"
          onclick={() => toggle(item.id)}
          class="flex w-full items-center justify-between px-3 py-2 text-left text-sm font-medium
            text-fg hover:bg-surface-hover"
        >
          {item.title}
          <svg
            width="12"
            height="12"
            viewBox="0 0 12 12"
            aria-hidden="true"
            class="transition-transform"
            class:rotate-180={expanded}
          >
            <path d="M2 4l4 4 4-4" stroke="currentColor" stroke-width="1.5" fill="none" />
          </svg>
        </button>
      </h3>
      {#if expanded}
        <div
          id="{id}-region-{item.id}"
          role="region"
          aria-labelledby="{id}-header-{item.id}"
          class="px-3 pb-3 text-sm"
        >
          {@render content(item.id)}
        </div>
      {/if}
    </div>
  {/each}
</div>
