<script lang="ts">
  import type { Snippet } from "svelte";
  import { resolve } from "$app/paths";
  import {
    commands,
    type HomeSummary_Serialize as HomeSummary,
    type Task_Serialize as Task,
  } from "$lib/api/bindings";
  import { call } from "$lib/api/client";
  import { errorMessage } from "$lib/i18n/errors";
  import { formatIsoDate, formatMoney } from "$lib/i18n/format";
  import { i18n } from "$lib/i18n/locale.svelte";
  import { m } from "$lib/paraglide/messages";
  import Button from "$lib/ui/Button.svelte";
  import { formatTimeInput, longDate } from "$lib/ui/date";
  import Spinner from "$lib/ui/Spinner.svelte";
  import { toast } from "$lib/ui/toast.svelte";
  import { itemTime } from "../calendar/format";
  import { localDate } from "../calendar/time";
  import { formatElapsed, timer, timerPanel } from "../time/timer.svelte";

  let data = $state<HomeSummary | null>(null);
  let now = $state(Date.now());

  $effect(() => {
    call(commands.homeSummary())
      .then((s) => (data = s))
      .catch((err) => toast.show({ kind: "error", message: errorMessage(err) }));
  });

  // The timer ticks while it runs.
  $effect(() => {
    if (!timer.running) return;
    now = Date.now();
    const id = setInterval(() => (now = Date.now()), 1000);
    return () => clearInterval(id);
  });

  let caption = $derived.by(() => {
    const r = timer.running;
    if (!r) return "";
    return [r.description, r.taskTitle ?? r.projectName ?? r.clientName]
      .filter(Boolean)
      .join(" · ");
  });

  function due(t: Task): string {
    const date = t.dueDate ? formatIsoDate(t.dueDate, i18n.locale) : "";
    return t.dueTime ? `${date} ${formatTimeInput(t.dueTime, i18n.locale)}` : date;
  }
</script>

{#snippet card(id: string, title: string, body: Snippet)}
  <section
    aria-labelledby={id}
    class="flex flex-col gap-3 rounded-[var(--radius)] border border-border p-4"
  >
    <h2 {id} class="text-sm font-semibold text-fg">{title}</h2>
    {@render body()}
  </section>
{/snippet}

{#snippet empty(text: string)}
  <p class="text-sm text-muted">{text}</p>
{/snippet}

{#snippet tasks(list: Task[], none: string)}
  {#if list.length === 0}
    {@render empty(none)}
  {:else}
    <ul class="flex flex-col gap-1">
      {#each list as t (t.id)}
        <li>
          <a
            href={resolve("/tasks/[id]", { id: t.id })}
            class="flex items-baseline justify-between gap-3 rounded px-2 py-1 text-sm hover:bg-surface-hover"
          >
            <span class="truncate text-fg">{t.title}</span>
            <span class="shrink-0 text-xs text-muted">{due(t)}</span>
          </a>
        </li>
      {/each}
    </ul>
  {/if}
{/snippet}

{#if !data}
  <Spinner />
{:else}
  {@const summary = data}
  <div class="grid gap-4 md:grid-cols-2 xl:grid-cols-3">
    {#snippet timerBody()}
      {#if timer.running}
        <div class="flex items-baseline gap-3">
          <span role="timer" class="font-mono text-2xl tabular-nums text-fg"
            >{formatElapsed(now - timer.running.startedAt)}</span
          >
          {#if caption}<span class="truncate text-sm text-muted">{caption}</span>{/if}
        </div>
      {:else}
        {@render empty(m.home_no_timer())}
        <div>
          <Button size="sm" onclick={() => (timerPanel.open = true)}>{m.timer_start()}</Button>
        </div>
      {/if}
    {/snippet}
    {@render card("home-timer", m.home_timer(), timerBody)}

    {#snippet overdueBody()}{@render tasks(summary.overdueTasks, m.home_no_overdue())}{/snippet}
    {@render card("home-overdue", m.home_overdue(), overdueBody)}

    {#snippet todayBody()}{@render tasks(summary.todayTasks, m.home_no_today())}{/snippet}
    {@render card("home-today", m.home_today(), todayBody)}

    {#snippet eventsBody()}
      {#if summary.events.length === 0}
        {@render empty(m.home_no_events())}
      {:else}
        <ul class="flex flex-col gap-1">
          {#each summary.events as e (`${e.id}:${e.startAt}`)}
            <li>
              <a
                href={resolve("/calendar")}
                class="flex flex-col rounded px-2 py-1 text-sm hover:bg-surface-hover"
              >
                <!-- The date is long: it goes under the title, not beside it. -->
                <span class="truncate text-fg">{e.title}</span>
                <span class="text-xs text-muted"
                  >{longDate(e.allDay ? e.start.slice(0, 10) : localDate(e.startAt), i18n.locale)}
                  {itemTime(e, i18n.locale)}</span
                >
              </a>
            </li>
          {/each}
        </ul>
      {/if}
    {/snippet}
    {@render card("home-events", m.home_events(), eventsBody)}

    {#snippet invoicesBody()}
      {#if summary.openInvoices.length === 0}
        {@render empty(m.home_no_invoices())}
      {:else}
        <ul class="flex flex-col gap-1">
          {#each summary.openInvoices as i (i.id)}
            <li>
              <a
                href="{resolve('/finance')}?invoice={encodeURIComponent(i.id)}"
                class="flex items-baseline gap-3 rounded px-2 py-1 text-sm hover:bg-surface-hover"
              >
                <span class="font-medium text-fg">{i.number}</span>
                <span class="flex-1 truncate text-muted">{i.clientName}</span>
                {#if i.overdue}
                  <span class="rounded-full bg-danger/15 px-2 text-xs text-danger"
                    >{m.invoice_status_overdue()}</span
                  >
                {/if}
                <span class="tabular-nums text-fg"
                  >{formatMoney(i.due.amountMinor, i.due.currency, i18n.locale)}</span
                >
              </a>
            </li>
          {/each}
        </ul>
      {/if}
    {/snippet}
    {@render card("home-invoices", m.home_invoices(), invoicesBody)}

    {#snippet clientsBody()}
      {#if summary.recentClients.length === 0}
        {@render empty(m.home_no_clients())}
      {:else}
        <ul class="flex flex-col gap-1">
          {#each summary.recentClients as c (c.id)}
            <li>
              <a
                href={resolve("/clients/[id]", { id: c.id })}
                class="block truncate rounded px-2 py-1 text-sm text-fg hover:bg-surface-hover"
                >{c.displayName}</a
              >
            </li>
          {/each}
        </ul>
      {/if}
    {/snippet}
    {@render card("home-clients", m.home_clients(), clientsBody)}
  </div>
{/if}
