<script lang="ts">
  import type { Snippet } from "svelte";
  import { rovingTarget } from "./roving";

  interface Tab {
    id: string;
    label: string;
    disabled?: boolean;
  }

  let {
    label,
    tabs,
    value = $bindable(""),
    panel,
  }: { label: string; tabs: Tab[]; value?: string; panel: Snippet<[string]> } = $props();

  const id = $props.id();
  const buttons: HTMLButtonElement[] = $state([]);

  function onkeydown(event: KeyboardEvent, index: number) {
    const target = rovingTarget(event.key, tabs, index);
    if (target === null) return;
    event.preventDefault();
    // Automatic activation: focusing a tab selects it.
    value = tabs[target].id;
    buttons[target]?.focus();
  }
</script>

<div class="flex flex-col">
  <div role="tablist" aria-label={label} class="flex gap-1 border-b border-border">
    {#each tabs as tab, index (tab.id)}
      <button
        bind:this={buttons[index]}
        id="{id}-tab-{tab.id}"
        type="button"
        role="tab"
        aria-selected={tab.id === value}
        aria-controls="{id}-panel"
        disabled={tab.disabled}
        tabindex={tab.id === value ? 0 : -1}
        onclick={() => (value = tab.id)}
        onkeydown={(e) => onkeydown(e, index)}
        class="-mb-px border-b-2 border-transparent px-3 py-1.5 text-sm text-muted
          hover:text-fg disabled:cursor-not-allowed disabled:opacity-50
          aria-selected:border-accent aria-selected:text-fg"
      >
        {tab.label}
      </button>
    {/each}
  </div>
  <div id="{id}-panel" role="tabpanel" aria-labelledby="{id}-tab-{value}" tabindex="0" class="pt-3">
    {@render panel(value)}
  </div>
</div>
