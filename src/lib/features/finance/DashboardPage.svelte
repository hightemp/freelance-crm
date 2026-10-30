<script lang="ts">
  import { resolve } from "$app/paths";
  import { commands, type Dashboard } from "$lib/api/bindings";
  import { call } from "$lib/api/client";
  import { errorMessage } from "$lib/i18n/errors";
  import { formatIsoDate, formatMoney } from "$lib/i18n/format";
  import { i18n } from "$lib/i18n/locale.svelte";
  import { m } from "$lib/paraglide/messages";
  import { todayIso } from "$lib/ui/date";
  import DateRangePicker from "$lib/ui/DateRangePicker.svelte";
  import RadioGroup from "$lib/ui/RadioGroup.svelte";
  import Spinner from "$lib/ui/Spinner.svelte";
  import { toast } from "$lib/ui/toast.svelte";
  import { monthRows, totalsFor } from "./dashboard";
  import MonthlyChart from "./MonthlyChart.svelte";

  let { today = todayIso() }: { today?: string } = $props();

  // `today` only seeds the period: the year so far.
  // svelte-ignore state_referenced_locally
  const seed = { today };
  let from = $state<string | null>(`${seed.today.slice(0, 4)}-01-01`);
  let to = $state<string | null>(seed.today);
  let data = $state<Dashboard | null>(null);
  /** The chosen currency; the default one until the user picks. */
  let picked = $state<string | null>(null);

  let totals = $derived(data ? totalsFor(data, picked) : null);
  let currency = $derived(totals?.currency ?? "");
  let rows = $derived(data && currency ? monthRows(data, currency) : []);
  let clients = $derived(data?.clients.filter((c) => c.currency === currency) ?? []);
  let tiles = $derived(
    totals
      ? ([
          { label: m.dashboard_income(), value: totals.incomeMinor },
          { label: m.dashboard_expenses(), value: totals.expensesMinor },
          { label: m.dashboard_profit(), value: totals.profitMinor },
          ...(totals.taxMinor != null && totals.netMinor != null
            ? [
                { label: m.dashboard_tax(), value: totals.taxMinor },
                { label: m.dashboard_net(), value: totals.netMinor },
              ]
            : []),
          { label: m.dashboard_unpaid(), value: totals.unpaidMinor },
          { label: m.dashboard_overdue(), value: totals.overdueMinor },
        ] satisfies { label: string; value: number }[])
      : [],
  );

  $effect(() => {
    if (!from || !to) return;
    const period = { from, to };
    void call(commands.financeDashboard(period))
      .then((d) => (data = d))
      .catch((err) => toast.show({ kind: "error", message: errorMessage(err) }));
  });
</script>

<section class="flex flex-col gap-6">
  <div class="flex flex-wrap items-end gap-6">
    <div class="w-96 max-w-full">
      <DateRangePicker
        startLabel={m.time_from()}
        endLabel={m.time_to()}
        bind:start={from}
        bind:end={to}
      />
    </div>
    {#if data && data.currencies.length > 1}
      <RadioGroup
        label={m.dashboard_currency()}
        options={data.currencies.map((c) => ({ value: c.currency, label: c.currency }))}
        bind:value={() => currency, (v) => (picked = v)}
      />
    {/if}
  </div>

  {#if !data || !totals}
    <Spinner />
  {:else}
    <div class="grid grid-cols-2 gap-3 sm:grid-cols-4 lg:grid-cols-7">
      {#each tiles as t (t.label)}
        <div
          role="group"
          aria-label={t.label}
          class="flex flex-col gap-1 rounded-md border border-border p-3"
        >
          <span class="text-xs text-muted">{t.label}</span>
          <span class="text-lg font-semibold tabular-nums text-fg"
            >{formatMoney(t.value, currency, i18n.locale)}</span
          >
        </div>
      {/each}
    </div>
    {#if data.taxesEnabled}
      <p class="text-xs text-muted">{m.taxes_estimate_note()}</p>
    {/if}

    <MonthlyChart {rows} {currency} />

    <div class="grid gap-6 lg:grid-cols-2">
      <section class="flex flex-col gap-2">
        <h2 id="dashboard-clients" class="text-sm font-medium text-fg">
          {m.dashboard_by_client()}
        </h2>
        {#if clients.length === 0}
          <p class="text-sm text-muted">{m.dashboard_no_income()}</p>
        {:else}
          <ul class="flex flex-col divide-y divide-border" aria-labelledby="dashboard-clients">
            {#each clients as c (c.clientId)}
              <li>
                <a
                  href="{resolve('/clients/[id]', { id: c.clientId })}?tab=finance"
                  class="flex justify-between gap-4 py-2 text-sm hover:bg-surface-hover"
                >
                  <span class="text-fg">{c.clientName}</span>
                  <span class="tabular-nums text-fg"
                    >{formatMoney(c.incomeMinor, c.currency, i18n.locale)}</span
                  >
                </a>
              </li>
            {/each}
          </ul>
        {/if}
      </section>

      <section class="flex flex-col gap-2">
        <h2 id="dashboard-open" class="text-sm font-medium text-fg">
          {m.dashboard_open_invoices()}
        </h2>
        {#if data.openInvoices.length === 0}
          <p class="text-sm text-muted">{m.dashboard_nothing_due()}</p>
        {:else}
          <ul class="flex flex-col divide-y divide-border" aria-labelledby="dashboard-open">
            {#each data.openInvoices as i (i.id)}
              <li class="flex items-center gap-3 py-2 text-sm">
                <span class="font-medium text-fg">{i.number}</span>
                <span class="flex-1 truncate text-muted">{i.clientName}</span>
                {#if i.overdue}
                  <span class="rounded-full bg-danger/15 px-2 py-0.5 text-xs text-danger"
                    >{m.invoice_status_overdue()}</span
                  >
                {/if}
                {#if i.dueDate}
                  <span class="text-muted">{formatIsoDate(i.dueDate, i18n.locale)}</span>
                {/if}
                <span class="tabular-nums text-fg"
                  >{formatMoney(i.due.amountMinor, i.due.currency, i18n.locale)}</span
                >
              </li>
            {/each}
          </ul>
        {/if}
      </section>
    </div>
  {/if}
</section>
