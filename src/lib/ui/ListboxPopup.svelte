<script lang="ts">
  import { m } from "$lib/paraglide/messages";
  import { floating, portal } from "./overlay";

  // Popup list for Select, Combobox and TagInput. Focus stays on the owning
  // control; the active option is exposed via aria-activedescendant.
  let {
    id,
    labelledby,
    items,
    active,
    isSelected,
    anchor,
    onpick,
  }: {
    id: string;
    labelledby: string;
    items: { label: string; disabled?: boolean }[];
    active: number;
    isSelected: (index: number) => boolean;
    anchor: HTMLElement | undefined;
    onpick: (index: number) => void;
  } = $props();
</script>

<div
  {@attach portal}
  {@attach floating(() => anchor)}
  {id}
  role="listbox"
  aria-labelledby={labelledby}
  tabindex="-1"
  class="z-50 max-h-64 min-w-48 overflow-auto rounded-[var(--radius)] border border-border bg-bg p-1
    shadow-lg"
>
  {#each items as item, index (index)}
    <!-- Options are picked by mouse here; keyboard is handled by the owning control. -->
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <div
      id="{id}-opt-{index}"
      role="option"
      aria-selected={isSelected(index)}
      aria-disabled={item.disabled || undefined}
      tabindex="-1"
      onmousedown={(e) => e.preventDefault()}
      onclick={() => !item.disabled && onpick(index)}
      class="cursor-default rounded-[4px] px-2 py-1.5 text-sm text-fg aria-disabled:opacity-50
        aria-selected:font-medium {index === active ? 'bg-surface-hover' : ''}"
    >
      {item.label}
    </div>
  {:else}
    <div class="px-2 py-1.5 text-sm text-muted">{m.ui_no_results()}</div>
  {/each}
</div>
