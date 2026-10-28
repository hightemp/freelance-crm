<script lang="ts">
  let {
    label,
    value,
    max = 100,
  }: { label: string; value?: number | null; max?: number } = $props();

  let clamped = $derived(value == null ? null : Math.min(Math.max(value, 0), max));
</script>

<div
  role="progressbar"
  aria-label={label}
  aria-valuemin={0}
  aria-valuemax={max}
  aria-valuenow={clamped ?? undefined}
  class="h-1.5 w-full overflow-hidden rounded-full bg-surface"
>
  {#if clamped === null}
    <div class="h-full w-1/3 animate-pulse rounded-full bg-accent"></div>
  {:else}
    <div class="h-full rounded-full bg-accent" style:width="{(clamped / max) * 100}%"></div>
  {/if}
</div>
