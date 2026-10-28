<script lang="ts">
  import { isItem, typeaheadMatch, type MenuEntry } from "./menu";
  import { afterPortal } from "./overlay";
  import { rovingTarget } from "./roving";

  export type CloseReason = "select" | "escape" | "tab" | "outside";

  let {
    items,
    onSelect,
    onClose,
    id,
  }: {
    items: MenuEntry[];
    onSelect: (id: string) => void;
    onClose: (reason: CloseReason) => void;
    id?: string;
  } = $props();

  let menuItems = $derived(items.filter(isItem));
  const buttons: HTMLButtonElement[] = $state([]);
  let active = $state(0);
  let query = "";
  let queryTimer: ReturnType<typeof setTimeout> | undefined;

  function focusItem(index: number) {
    if (index < 0) return;
    active = index;
    buttons[index]?.focus();
  }

  $effect(() => {
    afterPortal(() => focusItem(menuItems.findIndex((item) => !item.disabled)));
    return () => clearTimeout(queryTimer);
  });

  function select(index: number) {
    const item = menuItems[index];
    if (item.disabled) return;
    onSelect(item.id);
    onClose("select");
  }

  function onkeydown(event: KeyboardEvent) {
    const target = rovingTarget(event.key, menuItems, active);
    if (target !== null && event.key !== "ArrowLeft" && event.key !== "ArrowRight") {
      event.preventDefault();
      focusItem(target);
    } else if (event.key === "Escape") {
      event.preventDefault();
      event.stopPropagation();
      onClose("escape");
    } else if (event.key === "Tab") {
      event.preventDefault();
      onClose("tab");
    } else if (event.key.length === 1 && event.key !== " " && !event.ctrlKey && !event.metaKey) {
      clearTimeout(queryTimer);
      query += event.key;
      queryTimer = setTimeout(() => (query = ""), 500);
      focusItem(typeaheadMatch(menuItems, query, active));
    }
  }
</script>

<div
  {id}
  role="menu"
  tabindex="-1"
  {onkeydown}
  class="z-50 flex min-w-40 flex-col rounded-[var(--radius)] border border-border bg-bg p-1 shadow-lg"
>
  {#each items as entry, i (isItem(entry) ? entry.id : `sep-${i}`)}
    {#if isItem(entry)}
      {@const index = menuItems.indexOf(entry)}
      <button
        bind:this={buttons[index]}
        type="button"
        role="menuitem"
        tabindex="-1"
        disabled={entry.disabled}
        aria-disabled={entry.disabled || undefined}
        onclick={() => select(index)}
        class="rounded-[4px] px-2 py-1.5 text-left text-sm hover:bg-surface-hover
          focus-visible:bg-surface-hover focus-visible:outline-none
          disabled:cursor-not-allowed disabled:opacity-50
          {entry.danger ? 'text-danger' : 'text-fg'}"
      >
        {entry.label}
      </button>
    {:else}
      <div role="separator" class="my-1 h-px bg-border"></div>
    {/if}
  {/each}
</div>
