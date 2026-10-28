<script lang="ts">
  let {
    label,
    checked = $bindable(false),
    disabled = false,
  }: { label: string; checked?: boolean; disabled?: boolean } = $props();

  const id = $props.id();
</script>

<div class="inline-flex items-center gap-2">
  <button
    {id}
    type="button"
    role="switch"
    aria-checked={checked}
    aria-labelledby="{id}-label"
    {disabled}
    onclick={() => (checked = !checked)}
    class="relative h-5 w-9 rounded-full border border-border bg-surface transition-colors
      disabled:cursor-not-allowed disabled:opacity-50 aria-checked:border-accent aria-checked:bg-accent"
  >
    <span
      class="absolute top-0.5 left-0.5 size-3.5 rounded-full bg-bg shadow transition-transform"
      class:translate-x-4={checked}
    ></span>
  </button>
  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
  <span
    id="{id}-label"
    class="text-sm text-fg select-none"
    class:opacity-50={disabled}
    onclick={() => !disabled && (checked = !checked)}>{label}</span
  >
</div>
