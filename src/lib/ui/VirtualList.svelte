<script lang="ts" generics="T">
  import type { Snippet } from "svelte";

  // Fixed-height rows; only the visible window (plus overscan) is in the DOM.
  let {
    label,
    items,
    key,
    rowHeight,
    height,
    overscan = 5,
    row,
  }: {
    label: string;
    items: T[];
    key: (item: T) => string | number;
    rowHeight: number;
    /** Viewport height in pixels; by default the list fills its parent. */
    height?: number;
    /** Extra rows rendered beyond each edge of the viewport. */
    overscan?: number;
    row: Snippet<[T, number]>;
  } = $props();

  let scrollTop = $state(0);
  let measured = $state(0);
  // Without layout (e.g. before the first paint) assume a typical viewport.
  let viewport = $derived(height ?? (measured || 600));

  let first = $derived(Math.max(0, Math.floor(scrollTop / rowHeight) - overscan));
  let last = $derived(
    Math.min(items.length, Math.ceil((scrollTop + viewport) / rowHeight) + overscan),
  );
  let visible = $derived(items.slice(first, last).map((item, i) => ({ item, index: first + i })));
</script>

<div
  role="list"
  aria-label={label}
  class="relative overflow-y-auto"
  style:height={height === undefined ? "100%" : `${height}px`}
  bind:clientHeight={measured}
  onscroll={(e) => (scrollTop = e.currentTarget.scrollTop)}
>
  <div style:height="{items.length * rowHeight}px">
    {#each visible as { item, index } (key(item))}
      <div
        role="listitem"
        aria-setsize={items.length}
        aria-posinset={index + 1}
        class="absolute inset-x-0"
        style:top="{index * rowHeight}px"
        style:height="{rowHeight}px"
      >
        {@render row(item, index)}
      </div>
    {/each}
  </div>
</div>
