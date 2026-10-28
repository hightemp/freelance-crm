<script lang="ts">
  let {
    label,
    checked = $bindable(false),
    indeterminate = false,
    disabled = false,
  }: {
    label: string;
    checked?: boolean;
    indeterminate?: boolean;
    disabled?: boolean;
  } = $props();

  // A mixed checkbox becomes a regular one after the first toggle.
  let mixed = $derived(indeterminate);
  const id = $props.id();

  function toggle() {
    checked = mixed ? true : !checked;
    mixed = false;
  }
</script>

<div class="inline-flex items-center gap-2">
  <!-- A <button> gets Enter and Space activation for free. -->
  <button
    {id}
    type="button"
    role="checkbox"
    aria-checked={mixed ? "mixed" : checked}
    aria-labelledby="{id}-label"
    {disabled}
    onclick={toggle}
    class="grid size-4 place-items-center rounded-[4px] border border-border bg-bg text-accent-fg
      disabled:cursor-not-allowed disabled:opacity-50
      aria-checked:border-accent aria-checked:bg-accent
      aria-[checked=mixed]:border-accent aria-[checked=mixed]:bg-accent"
  >
    {#if mixed}
      <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden="true">
        <path d="M2 5h6" stroke="currentColor" stroke-width="2" />
      </svg>
    {:else if checked}
      <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden="true">
        <path d="M1.5 5.5 4 8l4.5-6" stroke="currentColor" stroke-width="2" fill="none" />
      </svg>
    {/if}
  </button>
  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
  <span
    id="{id}-label"
    class="text-sm text-fg select-none"
    class:opacity-50={disabled}
    onclick={() => !disabled && toggle()}>{label}</span
  >
</div>
