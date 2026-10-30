<script lang="ts">
  import Pencil from "@lucide/svelte/icons/pencil";
  import Plus from "@lucide/svelte/icons/plus";
  import Trash from "@lucide/svelte/icons/trash-2";
  import {
    commands,
    type Expense_Serialize as Expense,
    type ExpenseFilter,
    type Project_Serialize as Project,
  } from "$lib/api/bindings";
  import { call } from "$lib/api/client";
  import { errorMessage } from "$lib/i18n/errors";
  import { formatIsoDate, formatMoney } from "$lib/i18n/format";
  import { i18n } from "$lib/i18n/locale.svelte";
  import { m } from "$lib/paraglide/messages";
  import Button from "$lib/ui/Button.svelte";
  import { todayIso } from "$lib/ui/date";
  import DateRangePicker from "$lib/ui/DateRangePicker.svelte";
  import { confirm } from "$lib/ui/dialogs.svelte";
  import Select from "$lib/ui/Select.svelte";
  import Spinner from "$lib/ui/Spinner.svelte";
  import { toast } from "$lib/ui/toast.svelte";
  import ExpenseDialog from "./ExpenseDialog.svelte";

  let { today = todayIso() }: { today?: string } = $props();

  const ALL = "";
  // `today` only seeds the period: the month so far.
  // svelte-ignore state_referenced_locally
  const seed = { today };
  let from = $state<string | null>(`${seed.today.slice(0, 8)}01`);
  let to = $state<string | null>(seed.today);
  let category = $state<string | null>(null);
  let expenses = $state<Expense[] | null>(null);
  let categories = $state<string[]>([]);
  let projects = $state<Project[]>([]);
  /** The expense in the dialog: `null` for a new one. */
  let editing = $state<{ expense: Expense | null } | null>(null);

  const showError = (err: unknown) => toast.show({ kind: "error", message: errorMessage(err) });

  let filter = $derived<ExpenseFilter>({ projectId: null, from, to, category });
  let totals = $derived.by(() => {
    const sums: Record<string, number> = {};
    for (const e of expenses ?? [])
      sums[e.amount.currency] = (sums[e.amount.currency] ?? 0) + e.amount.amountMinor;
    return Object.entries(sums).map(([currency, minor]) =>
      formatMoney(minor, currency, i18n.locale),
    );
  });

  async function load() {
    try {
      const f = { ...filter };
      [expenses, categories] = await Promise.all([
        call(commands.listExpenses(f)),
        call(commands.expenseCategories()),
      ]);
    } catch (err) {
      showError(err);
    }
  }

  $effect(() => {
    void filter;
    void load();
  });

  $effect(() => {
    void call(commands.listProjects({ query: null, status: null, clientId: null, archived: false }))
      .then((p) => (projects = p))
      .catch(showError);
  });

  async function remove(e: Expense) {
    const ok = await confirm({
      title: m.expense_delete_title(),
      confirmLabel: m.ui_delete(),
      danger: true,
    });
    if (!ok) return;
    try {
      await call(commands.deleteExpense(e.id));
      await load();
    } catch (err) {
      showError(err);
    }
  }
</script>

<section class="flex flex-col gap-4">
  <div class="flex flex-wrap items-end gap-4">
    <div class="w-96 max-w-full">
      <DateRangePicker
        startLabel={m.time_from()}
        endLabel={m.time_to()}
        bind:start={from}
        bind:end={to}
      />
    </div>
    <div class="w-56">
      <Select
        label={m.expense_category()}
        options={[
          { value: ALL, label: m.expense_all_categories() },
          ...categories.map((c) => ({ value: c, label: c })),
        ]}
        bind:value={() => category ?? ALL, (v) => (category = v || null)}
      />
    </div>
    <Button variant="primary" onclick={() => (editing = { expense: null })}>
      <Plus class="size-4" aria-hidden="true" />
      {m.expense_add()}
    </Button>
  </div>

  {#if expenses === null}
    <Spinner />
  {:else if expenses.length === 0}
    <p class="text-sm text-muted">{m.expense_list_empty()}</p>
  {:else}
    <table class="w-full text-sm">
      <thead class="text-left text-muted">
        <tr>
          <th class="py-2 font-medium">{m.expense_date()}</th>
          <th class="py-2 font-medium">{m.expense_category()}</th>
          <th class="py-2 font-medium">{m.expense_description()}</th>
          <th class="py-2 font-medium">{m.invoice_project()}</th>
          <th class="py-2 text-right font-medium">{m.expense_amount()}</th>
          <th class="py-2"><span class="sr-only">{m.invoice_col_actions()}</span></th>
        </tr>
      </thead>
      <tbody class="divide-y divide-border">
        {#each expenses as e (e.id)}
          <tr>
            <td class="py-2 text-muted">{formatIsoDate(e.spentOn, i18n.locale)}</td>
            <td class="py-2 text-fg">{e.category}</td>
            <td class="py-2 text-fg">{e.description}</td>
            <td class="py-2 text-muted">{e.projectName ?? ""}</td>
            <td class="py-2 text-right tabular-nums text-fg"
              >{formatMoney(e.amount.amountMinor, e.amount.currency, i18n.locale)}</td
            >
            <td class="py-2 text-right whitespace-nowrap">
              <Button
                variant="ghost"
                size="sm"
                aria-label={m.expense_edit_named({ name: e.description || e.category })}
                onclick={() => (editing = { expense: e })}
              >
                <Pencil class="size-4" aria-hidden="true" />
              </Button>
              <Button
                variant="ghost"
                size="sm"
                aria-label={m.expense_delete_named({ name: e.description || e.category })}
                onclick={() => remove(e)}
              >
                <Trash class="size-4" aria-hidden="true" />
              </Button>
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
    <p class="text-right text-sm font-medium text-fg">
      {m.expense_total({ amounts: totals.join(" · ") })}
    </p>
  {/if}
</section>

{#if editing}
  <ExpenseDialog
    expense={editing.expense}
    {projects}
    {categories}
    {today}
    onsaved={() => {
      editing = null;
      void load();
    }}
    onclose={() => (editing = null)}
  />
{/if}
