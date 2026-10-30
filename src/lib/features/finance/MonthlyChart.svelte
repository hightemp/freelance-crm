<script lang="ts">
  import { BarChart } from "layerchart";
  import { formatMoney } from "$lib/i18n/format";
  import { i18n } from "$lib/i18n/locale.svelte";
  import { m } from "$lib/paraglide/messages";
  import type { MonthRow } from "./dashboard";

  let { rows, currency }: { rows: MonthRow[]; currency: string } = $props();

  const monthLabel = (month: string) => {
    const [year, mon] = month.split("-").map(Number);
    return new Intl.DateTimeFormat(i18n.locale, { month: "short", year: "2-digit" }).format(
      new Date(year, mon - 1, 1),
    );
  };
  const amount = (major: number) =>
    formatMoney(Math.round(major * 10 ** digits), currency, i18n.locale);
  let digits = $derived(
    new Intl.NumberFormat("en", { style: "currency", currency }).resolvedOptions()
      .maximumFractionDigits ?? 2,
  );
  let series = $derived([
    { key: "income", label: m.dashboard_income(), color: "var(--color-accent)" },
    { key: "expenses", label: m.dashboard_expenses(), color: "var(--color-danger)" },
  ]);
</script>

<figure class="flex flex-col gap-2">
  <figcaption class="text-sm font-medium text-fg">{m.dashboard_by_month()}</figcaption>
  <div class="h-64" aria-hidden="true">
    <BarChart
      data={rows}
      x="month"
      {series}
      seriesLayout="group"
      legend
      props={{ xAxis: { format: monthLabel } }}
    />
  </div>
  <!-- The same numbers for screen readers. -->
  <table class="sr-only">
    <caption>{m.dashboard_by_month()}</caption>
    <thead>
      <tr>
        <th scope="col">{m.dashboard_month()}</th>
        {#each series as s (s.key)}<th scope="col">{s.label}</th>{/each}
      </tr>
    </thead>
    <tbody>
      {#each rows as row (row.month)}
        <tr>
          <th scope="row">{monthLabel(row.month)}</th>
          <td>{amount(row.income)}</td>
          <td>{amount(row.expenses)}</td>
        </tr>
      {/each}
    </tbody>
  </table>
</figure>
