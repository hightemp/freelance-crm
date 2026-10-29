<script lang="ts">
  import type { CalendarItem_Serialize as CalendarItem } from "$lib/api/bindings";
  import { i18n } from "$lib/i18n/locale.svelte";
  import { m } from "$lib/paraglide/messages";
  import { formatTimeInput } from "$lib/ui/date";
  import { clock, kindClass, shortDay } from "./format";
  import { layoutDay, weekSegments } from "./layout";
  import { itemDays, midnight } from "./time";

  let {
    days,
    today,
    items,
    onopen,
    onslot,
  }: {
    days: string[];
    today: string;
    items: CalendarItem[];
    onopen: (item: CalendarItem) => void;
    /** Local `YYYY-MM-DDTHH:MM` of the clicked slot. */
    onslot: (start: string) => void;
  } = $props();

  const SLOT_MINUTES = 30;
  const SLOTS = (24 * 60) / SLOT_MINUTES;
  const pad = (n: number) => String(n).padStart(2, "0");
  const slotTime = (slot: number) =>
    `${pad(Math.floor((slot * SLOT_MINUTES) / 60))}:${pad((slot * SLOT_MINUTES) % 60)}`;

  // All-day and multi-day items go to the top row; timed ones to the grid.
  let keyed = $derived(items.map((item, i) => ({ key: String(i), item, days: itemDays(item) })));
  let top = $derived(keyed.filter((k) => k.item.allDay || k.days.start !== k.days.end));
  let timed = $derived(keyed.filter((k) => !k.item.allDay && k.days.start === k.days.end));
  let segments = $derived(
    weekSegments(
      top.map((k) => ({ id: k.key, ...k.days })),
      days,
    ),
  );
  let lanes = $derived(Math.max(0, ...segments.map((s) => s.lane + 1)));

  function dayItems(date: string) {
    const start = midnight(date);
    const inDay = timed.filter((k) => k.days.start === date);
    const spans = inDay.map((k) => ({
      id: k.key,
      start: (k.item.startAt - start) / 60_000,
      end: (k.item.endAt - start) / 60_000,
    }));
    const placed = layoutDay(spans);
    return inDay.map((k, i) => ({ ...k, span: spans[i], place: placed.get(k.key)! }));
  }

  let body = $state<HTMLElement>();
  // Start scrolled to the working day.
  $effect(() => {
    if (body) body.scrollTop = (8 * 60 * body.scrollHeight) / (24 * 60);
  });
</script>

<div class="flex flex-col rounded-[var(--radius)] border border-border">
  <div
    class="grid border-b border-border"
    style:grid-template-columns="3.5rem repeat({days.length}, minmax(0, 1fr))"
  >
    <div></div>
    {#each days as date (date)}
      <div
        class={["px-2 py-1 text-xs", date === today ? "font-semibold text-accent" : "text-muted"]}
      >
        {shortDay(date, i18n.locale)}
      </div>
    {/each}
  </div>
  {#if lanes > 0}
    <div
      class="grid border-b border-border py-1"
      style:grid-template-columns="3.5rem repeat({days.length}, minmax(0, 1fr))"
      style:grid-template-rows="repeat({lanes}, 1.5rem)"
    >
      <span class="self-center px-1 text-[10px] text-muted" style:grid-row="1"
        >{m.calendar_all_day()}</span
      >
      {#each segments as s (s.id)}
        {@const item = top.find((k) => k.key === s.id)!.item}
        <button
          type="button"
          class={[
            "mx-1 truncate rounded px-1.5 text-left text-xs leading-5 focus-visible:outline-2 focus-visible:outline-focus",
            kindClass(item),
          ]}
          style:grid-column="{s.column + 2} / span {s.span}"
          style:grid-row={s.lane + 1}
          onclick={() => onopen(item)}
        >
          {item.title}
        </button>
      {/each}
    </div>
  {/if}
  <div bind:this={body} class="max-h-[36rem] overflow-y-auto">
    <div class="grid" style:grid-template-columns="3.5rem repeat({days.length}, minmax(0, 1fr))">
      <div aria-hidden="true">
        {#each Array.from({ length: 24 }, (_, h) => h) as hour (hour)}
          <div class="h-12 pr-1 text-right text-[10px] text-muted tabular-nums">
            {formatTimeInput(`${pad(hour)}:00`, i18n.locale)}
          </div>
        {/each}
      </div>
      {#each days as date (date)}
        <div class="relative border-l border-border">
          {#each Array.from({ length: SLOTS }, (_, s) => s) as slot (slot)}
            <button
              type="button"
              tabindex="-1"
              aria-label={m.calendar_new_at({
                time: `${shortDay(date, i18n.locale)}, ${formatTimeInput(slotTime(slot), i18n.locale)}`,
              })}
              class={[
                "block h-6 w-full hover:bg-surface/60",
                slot % 2 === 1 && "border-b border-border/60",
              ]}
              onclick={() => onslot(`${date}T${slotTime(slot)}`)}
            ></button>
          {/each}
          {#each dayItems(date) as { key, item, span, place } (key)}
            <button
              type="button"
              class={[
                "absolute overflow-hidden rounded px-1 text-left text-xs leading-4 focus-visible:outline-2 focus-visible:outline-focus",
                kindClass(item),
              ]}
              style:top="{(span.start / 1440) * 100}%"
              style:height="{(Math.max(span.end - span.start, 20) / 1440) * 100}%"
              style:left="{(place.column / place.columns) * 100}%"
              style:width="{100 / place.columns}%"
              onclick={() => onopen(item)}
            >
              <span class="tabular-nums text-muted">{clock(item.startAt, i18n.locale)}</span>
              {item.title}
            </button>
          {/each}
        </div>
      {/each}
    </div>
  </div>
</div>
