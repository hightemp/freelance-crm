<script lang="ts">
  import DashboardPage from "$lib/features/finance/DashboardPage.svelte";
  import ExpensesPage from "$lib/features/finance/ExpensesPage.svelte";
  import InvoicesPage from "$lib/features/finance/InvoicesPage.svelte";
  import { m } from "$lib/paraglide/messages";
  import Tabs from "$lib/ui/Tabs.svelte";

  let tab = $state("dashboard");
  let tabs = $derived([
    { id: "dashboard", label: m.finance_tab_dashboard() },
    { id: "invoices", label: m.finance_tab_invoices() },
    { id: "expenses", label: m.finance_tab_expenses() },
  ]);
</script>

<div class="flex flex-col gap-6 p-6">
  <h1 class="text-xl font-semibold text-fg">{m.nav_finance()}</h1>
  <Tabs label={m.finance_tabs()} {tabs} bind:value={tab}>
    {#snippet panel(current)}
      <div class="pt-4">
        {#if current === "dashboard"}
          <DashboardPage />
        {:else if current === "invoices"}
          <InvoicesPage />
        {:else}
          <ExpensesPage />
        {/if}
      </div>
    {/snippet}
  </Tabs>
</div>
