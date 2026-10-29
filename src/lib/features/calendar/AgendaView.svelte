<script lang="ts">
  import type { CalendarItem_Serialize as CalendarItem } from "$lib/api/bindings";
  import { i18n } from "$lib/i18n/locale.svelte";
  import { m } from "$lib/paraglide/messages";
  import { longDate } from "$lib/ui/date";
  import { itemTime, kindClass } from "./format";
  import { itemDays } from "./time";

  let {
    days,
    items,
    onopen,
  }: { days: string[]; items: CalendarItem[]; onopen: (item: CalendarItem) => void } = $props();

  // Every day an item covers lists it.
  let groups = $derived(
    days
      .map((date) => ({
        date,
        items: items.filter((item) => {
          const span = itemDays(item);
          return span.start <= date && span.end >= date;
        }),
      }))
      .filter((g) => g.items.length > 0),
  );
</script>

{#if groups.length === 0}
  <p class="text-sm text-muted">{m.calendar_empty()}</p>
{:else}
  <div class="flex flex-col gap-4">
    {#each groups as group (group.date)}
      <section class="flex flex-col gap-1">
        <h3 class="text-sm font-semibold text-fg">{longDate(group.date, i18n.locale)}</h3>
        <ul class="flex flex-col gap-1">
          {#each group.items as item, i (i)}
            <li>
              <button
                type="button"
                class={[
                  "flex w-full gap-3 rounded px-2 py-1 text-left text-sm focus-visible:outline-2 focus-visible:outline-focus",
                  kindClass(item),
                ]}
                onclick={() => onopen(item)}
              >
                <span class="w-12 shrink-0 tabular-nums text-muted"
                  >{itemTime(item, i18n.locale) || m.calendar_all_day()}</span
                >
                <span class="min-w-0 flex-1 truncate">{item.title}</span>
                {#if item.clientName}<span class="shrink-0 text-muted">{item.clientName}</span>{/if}
              </button>
            </li>
          {/each}
        </ul>
      </section>
    {/each}
  </div>
{/if}
