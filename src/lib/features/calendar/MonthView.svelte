<script lang="ts">
  import type { CalendarItem_Serialize as CalendarItem } from "$lib/api/bindings";
  import { i18n } from "$lib/i18n/locale.svelte";
  import { m } from "$lib/paraglide/messages";
  import { weekdayNames } from "$lib/ui/date";
  import { uiPrefs } from "$lib/ui/prefs.svelte";
  import { itemTime, kindClass, shortDay } from "./format";
  import { hiddenPerDay, weekSegments } from "./layout";
  import { itemDays } from "./time";

  let {
    days,
    month,
    today,
    items,
    onopen,
    onslot,
    onmore,
  }: {
    /** 42 dates, six weeks. */
    days: string[];
    /** `YYYY-MM` shown; other days are dimmed. */
    month: string;
    today: string;
    items: CalendarItem[];
    onopen: (item: CalendarItem) => void;
    onslot: (date: string) => void;
    onmore: (date: string) => void;
  } = $props();

  /** Lanes of items shown per day; the rest goes behind “+N more”. */
  const LANES = 3;

  let weeks = $derived(
    Array.from({ length: days.length / 7 }, (_, w) => days.slice(w * 7, w * 7 + 7)),
  );
  let byKey = $derived(new Map(items.map((item, i) => [String(i), item])));
  let spans = $derived(items.map((item, i) => ({ id: String(i), ...itemDays(item) })));
</script>

<div class="flex flex-col rounded-[var(--radius)] border border-border">
  <div class="grid grid-cols-7 border-b border-border text-xs text-muted" aria-hidden="true">
    {#each weekdayNames(i18n.locale, uiPrefs.weekStart) as name (name)}
      <div class="px-2 py-1">{name}</div>
    {/each}
  </div>
  {#each weeks as week (week[0])}
    {@const segments = weekSegments(spans, week)}
    {@const hidden = hiddenPerDay(segments, LANES)}
    <div
      class="relative grid min-h-28 grid-cols-7 grid-rows-[auto_repeat(4,minmax(1.5rem,auto))] border-b border-border last:border-b-0"
    >
      {#each week as date, col (date)}
        <button
          type="button"
          tabindex="-1"
          aria-label={m.calendar_new_on({ date: shortDay(date, i18n.locale) })}
          class="border-r border-border last:border-r-0 hover:bg-surface/60"
          style:grid-column={col + 1}
          style:grid-row="1 / -1"
          onclick={() => onslot(date)}
        ></button>
        <span
          class={[
            "pointer-events-none z-10 justify-self-start px-2 pt-1 text-xs tabular-nums",
            date.slice(0, 7) === month ? "text-fg" : "text-muted",
            date === today && "font-semibold text-accent",
          ]}
          style:grid-column={col + 1}
          style:grid-row="1"
        >
          {Number(date.slice(8))}
        </span>
        {#if hidden[col] > 0}
          <button
            type="button"
            class="z-10 justify-self-start px-2 text-xs text-muted hover:text-fg focus-visible:outline-2 focus-visible:outline-focus"
            style:grid-column={col + 1}
            style:grid-row={LANES + 2}
            onclick={() => onmore(date)}
          >
            {m.calendar_more({ count: hidden[col] })}
          </button>
        {/if}
      {/each}
      {#each segments.filter((s) => s.lane < LANES) as s (s.id)}
        {@const item = byKey.get(s.id)!}
        <button
          type="button"
          class={[
            "z-10 mx-1 my-0.5 truncate rounded px-1.5 text-left text-xs leading-5 focus-visible:outline-2 focus-visible:outline-focus",
            kindClass(item),
            s.continuesBefore && "rounded-l-none",
            s.continuesAfter && "rounded-r-none",
          ]}
          style:grid-column="{s.column + 1} / span {s.span}"
          style:grid-row={s.lane + 2}
          onclick={() => onopen(item)}
        >
          {#if itemTime(item, i18n.locale) && s.span === 1}<span class="tabular-nums text-muted"
              >{itemTime(item, i18n.locale)}</span
            >{/if}
          {item.title}
        </button>
      {/each}
    </div>
  {/each}
</div>
