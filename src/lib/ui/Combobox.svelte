<script lang="ts">
  import { m } from "$lib/paraglide/messages";
  import ListboxPopup from "./ListboxPopup.svelte";
  import type { Option } from "./options";
  import { rovingTarget } from "./roving";

  let {
    label,
    value = $bindable(null),
    options = [],
    load,
    onCreate,
    placeholder = "",
  }: {
    label: string;
    value?: string | null;
    /** Static options filtered locally. */
    options?: Option[];
    /** Async search; replaces `options` filtering when provided. */
    load?: (query: string) => Promise<Option[]>;
    /** Enables a "Create …" option for text without an exact match. */
    onCreate?: (label: string) => Promise<Option>;
    placeholder?: string;
  } = $props();

  const id = $props.id();
  let input = $state<HTMLInputElement>();
  // Options picked from async results or created, so their labels stay known.
  let known = $state<Option[]>([]);
  let loaded = $state<Option[]>([]);
  let selected = $derived([...options, ...known].find((o) => o.value === value) ?? null);
  let query = $state("");
  let dirty = $state(false);

  // Show the selected label unless the user is typing.
  $effect.pre(() => {
    const label = selected?.label ?? "";
    if (!dirty) query = label;
  });
  let open = $state(false);
  let active = $state(-1);
  let requestSeq = 0;

  let matches = $derived.by(() => {
    if (load) return loaded;
    const q = dirty ? query.trim().toLocaleLowerCase() : "";
    return options.filter((o) => o.label.toLocaleLowerCase().includes(q));
  });

  let canCreate = $derived(
    !!onCreate &&
      dirty &&
      query.trim() !== "" &&
      !matches.some((o) => o.label.toLocaleLowerCase() === query.trim().toLocaleLowerCase()),
  );

  let items = $derived([
    ...matches,
    ...(canCreate ? [{ value: "", label: m.ui_create({ value: query.trim() }) }] : []),
  ]);

  async function search(q: string) {
    if (!load) return;
    const seq = ++requestSeq;
    const result = await load(q);
    if (seq === requestSeq) loaded = result; // ignore stale responses
  }

  function show() {
    open = true;
    active = -1;
    if (load && !dirty) void search("");
  }

  function reset() {
    open = false;
    dirty = false;
    query = selected?.label ?? "";
  }

  function commit(option: Option) {
    if (!options.some((o) => o.value === option.value)) known = [...known, option];
    value = option.value;
    query = option.label;
    dirty = false;
    open = false;
  }

  async function pick(index: number) {
    if (index < matches.length) return commit(matches[index]);
    if (onCreate) commit(await onCreate(query.trim()));
  }

  function oninput() {
    dirty = true;
    open = true;
    active = -1;
    void search(query);
  }

  function onkeydown(event: KeyboardEvent) {
    if (event.key === "ArrowDown" && !open) {
      event.preventDefault();
      show();
      active = 0;
      return;
    }
    if (event.key === "Escape") {
      event.preventDefault();
      reset();
      return;
    }
    if (!open) return;
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      active = rovingTarget(event.key, items, active) ?? active;
    } else if (event.key === "Enter" && active >= 0) {
      event.preventDefault();
      void pick(active);
    }
  }
</script>

<div class="flex flex-col gap-1">
  <label id="{id}-label" for="{id}-input" class="text-sm font-medium text-fg">{label}</label>
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
    {placeholder}
    onclick={() => !open && show()}
    onblur={reset}
    {oninput}
    {onkeydown}
    class="h-8 w-full rounded-[var(--radius)] border border-border bg-bg px-2.5 text-sm text-fg
      placeholder:text-muted"
  />
  {#if open}
    <ListboxPopup
      id="{id}-listbox"
      labelledby="{id}-label"
      {items}
      {active}
      isSelected={(i) => i < matches.length && matches[i].value === value}
      anchor={input}
      onpick={pick}
    />
  {/if}
</div>
