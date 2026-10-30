<script lang="ts">
  import {
    commands,
    type Expense_Serialize as Expense,
    type ExpenseInput,
    type Project_Serialize as Project,
  } from "$lib/api/bindings";
  import { AppError, call } from "$lib/api/client";
  import { COMMON_CURRENCIES, currencyLabel } from "$lib/app/currencies";
  import { appSettings } from "$lib/app/settings.svelte";
  import { codeMessage, errorMessage } from "$lib/i18n/errors";
  import { i18n } from "$lib/i18n/locale.svelte";
  import { m } from "$lib/paraglide/messages";
  import Button from "$lib/ui/Button.svelte";
  import Combobox from "$lib/ui/Combobox.svelte";
  import DatePicker from "$lib/ui/DatePicker.svelte";
  import Dialog from "$lib/ui/Dialog.svelte";
  import MoneyField from "$lib/ui/MoneyField.svelte";
  import Select from "$lib/ui/Select.svelte";
  import TextField from "$lib/ui/TextField.svelte";
  import { todayIso } from "$lib/ui/date";
  import { toast } from "$lib/ui/toast.svelte";
  import AttachmentPanel from "../files/AttachmentPanel.svelte";

  let {
    expense = null,
    projects,
    categories,
    today = todayIso(),
    onsaved,
    onclose,
  }: {
    /** `null` for a new expense. */
    expense?: Expense | null;
    projects: Project[];
    /** Categories used before, offered as suggestions. */
    categories: string[];
    today?: string;
    onsaved: () => void;
    onclose: () => void;
  } = $props();

  const NONE = "";
  // The expense only seeds the form.
  // svelte-ignore state_referenced_locally
  const seed = { expense, today };
  let draft = $state({
    amount: seed.expense?.amount.amountMinor ?? null,
    currency: seed.expense?.amount.currency ?? appSettings.value.defaultCurrency,
    spentOn: seed.expense?.spentOn ?? seed.today,
    category: seed.expense?.category ?? null,
    projectId: seed.expense?.projectId ?? null,
    description: seed.expense?.description ?? "",
  });
  let created = $state<string[]>([]);
  let errors = $state<Record<string, string>>({});
  let saving = $state(false);

  let categoryOptions = $derived(
    [...new Set([...categories, ...created])].map((c) => ({ value: c, label: c })),
  );
  const error = (path: string) => (errors[path] ? codeMessage(errors[path]) : undefined);

  function toInput(): ExpenseInput {
    return {
      projectId: draft.projectId,
      category: draft.category ?? "",
      amount: { amountMinor: draft.amount ?? 0, currency: draft.currency },
      spentOn: draft.spentOn,
      description: draft.description,
    };
  }

  async function save(event: SubmitEvent) {
    event.preventDefault();
    errors = {
      ...(draft.amount === null ? { amount: "validation.required" } : {}),
      ...(draft.category ? {} : { category: "validation.required" }),
    };
    if (Object.keys(errors).length > 0) return;
    saving = true;
    try {
      if (seed.expense) await call(commands.updateExpense(seed.expense.id, toInput()));
      else await call(commands.createExpense(toInput()));
      onsaved();
    } catch (err) {
      const field = err instanceof AppError ? err.params.field : undefined;
      if (err instanceof AppError && typeof field === "string") errors = { [field]: err.code };
      else toast.show({ kind: "error", message: errorMessage(err) });
    } finally {
      saving = false;
    }
  }
</script>

<Dialog open title={expense ? m.expense_edit() : m.expense_new()} {onclose}>
  <form
    id="expense-form"
    class="flex w-[32rem] max-w-full flex-col gap-4"
    novalidate
    onsubmit={save}
  >
    <div class="grid gap-3 sm:grid-cols-2">
      <MoneyField
        label={m.expense_amount()}
        currency={draft.currency}
        required
        error={error("amount")}
        bind:value={draft.amount}
      />
      <Select
        label={m.invoice_currency()}
        options={COMMON_CURRENCIES.map((code) => ({
          value: code,
          label: currencyLabel(code, i18n.locale),
        }))}
        bind:value={() => draft.currency, (v) => (draft.currency = v ?? draft.currency)}
      />
      <DatePicker
        label={m.expense_date()}
        required
        error={error("spentOn")}
        bind:value={() => draft.spentOn, (v) => (draft.spentOn = v ?? draft.spentOn)}
      />
      <div class="flex flex-col gap-1">
        <Combobox
          label={m.expense_category()}
          options={categoryOptions}
          onCreate={async (label) => {
            created = [...created, label];
            return { value: label, label };
          }}
          bind:value={draft.category}
        />
        {#if error("category")}
          <p class="text-xs text-danger" role="alert">{error("category")}</p>
        {/if}
      </div>
    </div>
    <Select
      label={m.invoice_project()}
      options={[
        { value: NONE, label: m.invoice_no_project() },
        ...projects.map((p) => ({ value: p.id, label: `${p.name} · ${p.clientName}` })),
      ]}
      bind:value={() => draft.projectId ?? NONE, (v) => (draft.projectId = v || null)}
    />
    <TextField label={m.expense_description()} bind:value={draft.description} />
  </form>
  {#if expense}
    <section class="mt-4 flex flex-col gap-2" aria-labelledby="expense-receipts">
      <h3 id="expense-receipts" class="text-sm font-medium text-fg">{m.expense_receipts()}</h3>
      <AttachmentPanel owner={{ type: "expense", id: expense.id }} />
    </section>
  {/if}
  {#snippet footer()}
    <Button onclick={onclose}>{m.ui_cancel()}</Button>
    <Button type="submit" form="expense-form" variant="primary" loading={saving}
      >{m.ui_save()}</Button
    >
  {/snippet}
</Dialog>
