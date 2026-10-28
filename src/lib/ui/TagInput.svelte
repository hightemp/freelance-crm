<script lang="ts">
  import { m } from "$lib/paraglide/messages";
  import ListboxPopup from "./ListboxPopup.svelte";
  import { rovingTarget } from "./roving";

  let {
    label,
    value = $bindable([]),
    suggestions = [],
  }: { label: string; value?: string[]; suggestions?: string[] } = $props();

  const id = $props.id();
  let input = $state<HTMLInputElement>();
  let query = $state("");
  let active = $state(-1);

  const has = (tag: string) => value.some((t) => t.toLocaleLowerCase() === tag.toLocaleLowerCase());

  let matches = $derived.by(() => {
    const q = query.trim().toLocaleLowerCase();
    if (!q) return [];
    return suggestions.filter((s) => !has(s) && s.toLocaleLowerCase().includes(q));
  });
  let open = $derived(matches.length > 0);

  function add(tag: string) {
    const t = tag.trim();
    if (t && !has(t)) value = [...value, t];
    query = "";
    active = -1;
  }

  function remove(tag: string) {
    value = value.filter((t) => t !== tag);
  }

  function onkeydown(event: KeyboardEvent) {
    if (event.key === "Enter") {
      event.preventDefault();
      add(active >= 0 ? matches[active] : query);
    } else if ((event.key === "ArrowDown" || event.key === "ArrowUp") && open) {
      event.preventDefault();
      active =
        rovingTarget(
          event.key,
          matches.map(() => ({})),
          active,
        ) ?? active;
    } else if (event.key === "Backspace" && query === "" && value.length > 0) {
      value = value.slice(0, -1);
    } else if (event.key === "Escape") {
      query = "";
    }
  }
</script>

<div class="flex flex-col gap-1">
  <label id="{id}-label" for="{id}-input" class="text-sm font-medium text-fg">{label}</label>
  <div
    class="flex min-h-8 flex-wrap items-center gap-1 rounded-[var(--radius)] border border-border
      bg-bg px-1.5 py-1"
  >
    {#each value as tag (tag)}
      <span class="inline-flex items-center gap-1 rounded bg-surface px-1.5 text-xs text-fg">
        {tag}
        <button
          type="button"
          aria-label={m.ui_remove({ value: tag })}
          onclick={() => remove(tag)}
          class="text-muted hover:text-fg"
        >
          <svg width="8" height="8" viewBox="0 0 12 12" aria-hidden="true">
            <path d="M2 2l8 8M10 2l-8 8" stroke="currentColor" stroke-width="2" />
          </svg>
        </button>
      </span>
    {/each}
    <input
      bind:this={input}
      bind:value={query}
      id="{id}-input"
      type="text"
      role="combobox"
      autocomplete="off"
      aria-autocomplete="list"
      aria-expanded={open}
      aria-controls={open ? `${id}-listbox` : undefined}
      aria-activedescendant={open && active >= 0 ? `${id}-listbox-opt-${active}` : undefined}
      oninput={() => (active = -1)}
      {onkeydown}
      class="min-w-20 flex-1 bg-transparent px-1 text-sm text-fg outline-none"
    />
  </div>
  {#if open}
    <ListboxPopup
      id="{id}-listbox"
      labelledby="{id}-label"
      items={matches.map((label) => ({ label }))}
      {active}
      isSelected={() => false}
      anchor={input}
      onpick={(i) => add(matches[i])}
    />
  {/if}
</div>
