<script lang="ts">
  import ListboxPopup from "./ListboxPopup.svelte";
  import { typeaheadMatch } from "./menu";
  import type { Option } from "./options";
  import { clickOutside } from "./overlay";
  import { firstEnabled, rovingTarget } from "./roving";

  let {
    label,
    options,
    value = $bindable(null),
    placeholder = "",
    disabled = false,
  }: {
    label: string;
    options: Option[];
    value?: string | null;
    placeholder?: string;
    disabled?: boolean;
  } = $props();

  const id = $props.id();
  let trigger = $state<HTMLButtonElement>();
  let open = $state(false);
  let active = $state(-1);
  let query = "";
  let queryTimer: ReturnType<typeof setTimeout> | undefined;

  let selectedIndex = $derived(options.findIndex((o) => o.value === value));

  function show() {
    open = true;
    active = selectedIndex >= 0 ? selectedIndex : firstEnabled(options);
  }

  function pick(index: number) {
    if (options[index]?.disabled) return;
    value = options[index].value;
    open = false;
    trigger?.focus();
  }

  function onkeydown(event: KeyboardEvent) {
    if (!open) {
      if (["ArrowDown", "ArrowUp", "Enter", " "].includes(event.key)) {
        event.preventDefault();
        show();
      }
      return;
    }
    const target = rovingTarget(event.key, options, active);
    if (target !== null) {
      event.preventDefault();
      active = target;
    } else if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      if (active >= 0) pick(active);
    } else if (event.key === "Escape" || event.key === "Tab") {
      if (event.key === "Escape") event.preventDefault();
      open = false;
    } else if (event.key.length === 1) {
      clearTimeout(queryTimer);
      query += event.key;
      queryTimer = setTimeout(() => (query = ""), 500);
      const match = typeaheadMatch(options, query, Math.max(active, 0));
      if (match >= 0) active = match;
    }
  }
</script>

<div class="flex flex-col gap-1">
  <span id="{id}-label" class="text-sm font-medium text-fg">{label}</span>
  <button
    bind:this={trigger}
    type="button"
    role="combobox"
    aria-labelledby="{id}-label"
    aria-haspopup="listbox"
    aria-expanded={open}
    aria-controls={open ? `${id}-listbox` : undefined}
    aria-activedescendant={open && active >= 0 ? `${id}-listbox-opt-${active}` : undefined}
    {disabled}
    onclick={() => (open ? (open = false) : show())}
    {onkeydown}
    class="flex h-8 w-full items-center justify-between gap-2 rounded-[var(--radius)] border
      border-border bg-bg px-2.5 text-left text-sm text-fg disabled:opacity-50"
  >
    <span class:text-muted={selectedIndex < 0} class="truncate">
      {selectedIndex >= 0 ? options[selectedIndex].label : placeholder}
    </span>
    <svg width="12" height="12" viewBox="0 0 12 12" aria-hidden="true" class="text-muted">
      <path d="M2 4l4 4 4-4" stroke="currentColor" stroke-width="1.5" fill="none" />
    </svg>
  </button>
  {#if open}
    <div
      class="contents"
      {@attach clickOutside(
        () => (open = false),
        () => [trigger, document.getElementById(`${id}-listbox`)],
      )}
    >
      <ListboxPopup
        id="{id}-listbox"
        labelledby="{id}-label"
        items={options}
        {active}
        isSelected={(i) => i === selectedIndex}
        anchor={trigger}
        onpick={pick}
      />
    </div>
  {/if}
</div>
