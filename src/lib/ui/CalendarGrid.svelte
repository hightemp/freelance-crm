<script lang="ts">
  import { tick, untrack } from "svelte";
  import { i18n } from "$lib/i18n/locale.svelte";
  import { m } from "$lib/paraglide/messages";
  import {
    addDays,
    addMonths,
    longDate,
    monthGrid,
    monthTitle,
    startOfWeek,
    todayIso,
    weekdayNames,
  } from "./date";
  import { afterPortal } from "./overlay";

  let {
    selected = null,
    today = todayIso(),
    weekStart = 1,
    min,
    max,
    autofocus = false,
    onselect,
  }: {
    selected?: string | null;
    today?: string;
    /** 0 = Sunday … 6 = Saturday. */
    weekStart?: number;
    min?: string;
    max?: string;
    autofocus?: boolean;
    onselect: (date: string) => void;
  } = $props();

  const id = $props.id();
  let root = $state<HTMLElement>();

  const clamp = (d: string) => (min && d < min ? min : max && d > max ? max : d);
  const outOfRange = (d: string) => (!!min && d < min) || (!!max && d > max);

  // ISO strings compare lexicographically, so plain `<` works for dates.
  // The grid starts at the selected day; afterwards focus moves independently.
  let focused = $state(untrack(() => clamp(selected ?? today)));
  let year = $derived(Number(focused.slice(0, 4)));
  let month = $derived(Number(focused.slice(5, 7)));
  let weeks = $derived(monthGrid(year, month, weekStart));

  async function moveTo(date: string, focus = true) {
    focused = clamp(date);
    if (!focus) return;
    await tick();
    root?.querySelector<HTMLElement>(`[data-date="${focused}"]`)?.focus();
  }

  $effect(() => {
    if (autofocus) afterPortal(() => void moveTo(focused));
  });

  function onkeydown(event: KeyboardEvent) {
    const moves: Record<string, () => string> = {
      ArrowLeft: () => addDays(focused, -1),
      ArrowRight: () => addDays(focused, 1),
      ArrowUp: () => addDays(focused, -7),
      ArrowDown: () => addDays(focused, 7),
      Home: () => startOfWeek(focused, weekStart),
      End: () => addDays(startOfWeek(focused, weekStart), 6),
      PageUp: () => addMonths(focused, event.shiftKey ? -12 : -1),
      PageDown: () => addMonths(focused, event.shiftKey ? 12 : 1),
    };
    const move = moves[event.key];
    if (!move) return;
    event.preventDefault();
    void moveTo(move());
  }
</script>

<div bind:this={root} class="flex flex-col gap-2">
  <div class="flex items-center justify-between">
    <button
      type="button"
      aria-label={m.ui_prev_month()}
      onclick={() => moveTo(addMonths(focused, -1), false)}
      class="grid size-7 place-items-center rounded-[var(--radius)] text-muted hover:bg-surface-hover"
    >
      <svg width="12" height="12" viewBox="0 0 12 12" aria-hidden="true">
        <path d="M8 2 4 6l4 4" stroke="currentColor" stroke-width="1.5" fill="none" />
      </svg>
    </button>
    <h2 id="{id}-title" aria-live="polite" class="text-sm font-medium text-fg capitalize">
      {monthTitle(focused, i18n.locale)}
    </h2>
    <button
      type="button"
      aria-label={m.ui_next_month()}
      onclick={() => moveTo(addMonths(focused, 1), false)}
      class="grid size-7 place-items-center rounded-[var(--radius)] text-muted hover:bg-surface-hover"
    >
      <svg width="12" height="12" viewBox="0 0 12 12" aria-hidden="true">
        <path d="m4 2 4 4-4 4" stroke="currentColor" stroke-width="1.5" fill="none" />
      </svg>
    </button>
  </div>
  <table role="grid" aria-labelledby="{id}-title" class="border-collapse" {onkeydown}>
    <thead>
      <tr>
        {#each weekdayNames(i18n.locale, weekStart) as name (name)}
          <th scope="col" class="size-8 text-xs font-normal text-muted">{name}</th>
        {/each}
      </tr>
    </thead>
    <tbody>
      {#each weeks as week (week[0])}
        <tr>
          {#each week as day (day)}
            {@const disabled = outOfRange(day)}
            <td role="gridcell" aria-selected={day === selected} class="p-0">
              <button
                type="button"
                data-date={day}
                tabindex={day === focused ? 0 : -1}
                aria-label={longDate(day, i18n.locale)}
                aria-current={day === today ? "date" : undefined}
                aria-disabled={disabled || undefined}
                onclick={() => !disabled && onselect(day)}
                class="size-8 rounded-[var(--radius)] text-sm hover:bg-surface-hover
                  aria-[current=date]:font-semibold aria-disabled:cursor-not-allowed
                  aria-disabled:opacity-40
                  {Number(day.slice(5, 7)) === month ? 'text-fg' : 'text-muted'}
                  {day === selected ? 'bg-accent text-accent-fg hover:bg-accent-hover' : ''}"
              >
                {Number(day.slice(8))}
              </button>
            </td>
          {/each}
        </tr>
      {/each}
    </tbody>
  </table>
</div>
