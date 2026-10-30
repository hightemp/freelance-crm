<script lang="ts">
  import Lock from "@lucide/svelte/icons/lock";
  import Plus from "@lucide/svelte/icons/plus";
  import Trash from "@lucide/svelte/icons/trash-2";
  import {
    commands,
    type Client_Serialize as Client,
    type Discount,
    type Invoice_Serialize as Invoice,
    type InvoiceInput,
    type Project_Serialize as Project,
    type StoredStatus,
  } from "$lib/api/bindings";
  import { AppError, call } from "$lib/api/client";
  import { COMMON_CURRENCIES, currencyLabel } from "$lib/app/currencies";
  import { appSettings } from "$lib/app/settings.svelte";
  import { codeMessage, errorMessage } from "$lib/i18n/errors";
  import { formatMoney } from "$lib/i18n/format";
  import { i18n } from "$lib/i18n/locale.svelte";
  import { m } from "$lib/paraglide/messages";
  import Button from "$lib/ui/Button.svelte";
  import DatePicker from "$lib/ui/DatePicker.svelte";
  import Dialog from "$lib/ui/Dialog.svelte";
  import MoneyField from "$lib/ui/MoneyField.svelte";
  import NumberField from "$lib/ui/NumberField.svelte";
  import Select from "$lib/ui/Select.svelte";
  import TextArea from "$lib/ui/TextArea.svelte";
  import TextField from "$lib/ui/TextField.svelte";
  import { todayIso } from "$lib/ui/date";
  import { toast } from "$lib/ui/toast.svelte";
  import { computeTotals, isLocked, statusLabel } from "./invoice";

  let {
    invoice = null,
    clients,
    projects,
    today = todayIso(),
    onsaved,
    onclose,
  }: {
    /** `null` for a new invoice. */
    invoice?: Invoice | null;
    clients: Client[];
    projects: Project[];
    today?: string;
    onsaved: (invoice: Invoice) => void;
    onclose: () => void;
  } = $props();

  const NONE = "";
  const DEFAULT = "";

  interface ItemDraft {
    key: number;
    description: string;
    /** Units, e.g. 1.5 hours. */
    quantity: number | null;
    unit: string;
    price: number | null;
  }

  let nextKey = 0;
  const item = (patch: Partial<ItemDraft> = {}): ItemDraft => ({
    key: nextKey++,
    description: "",
    quantity: 1,
    unit: "",
    price: null,
    ...patch,
  });

  // The invoice only seeds the form.
  // svelte-ignore state_referenced_locally
  const seed = invoice;
  // svelte-ignore state_referenced_locally
  const seedToday = today;
  let draft = $state({
    number: seed?.number ?? "",
    clientId: seed?.clientId ?? null,
    projectId: seed?.projectId ?? null,
    currency: seed?.currency ?? appSettings.value.defaultCurrency,
    issueDate: seed?.issueDate ?? seedToday,
    dueDate: seed?.dueDate ?? null,
    status: (seed?.storedStatus ?? "draft") as StoredStatus,
    items: seed
      ? seed.items.map((i) =>
          item({
            description: i.description,
            quantity: i.quantityMilli / 1000,
            unit: i.unit,
            price: i.price.amountMinor,
          }),
        )
      : [item()],
    discountType: (seed?.discount.type ?? "none") as Discount["type"],
    /** Percent (10.5) or minor units, by type. */
    discountValue: seed && seed.discount.type !== "none" ? discountValue(seed.discount) : null,
    taxPercent: seed ? seed.taxBp / 100 : 0,
    notesMd: seed?.notesMd ?? "",
    language: seed?.language ?? (i18n.locale === "ru" ? "ru" : "en"),
    showSignature: seed?.showSignature ?? null,
  });
  let numberPreview = $state("");
  let errors = $state<Record<string, string>>({});
  let saving = $state(false);

  function discountValue(d: Discount): number | null {
    if (d.type === "percent") return d.value / 100;
    if (d.type === "fixed") return d.value;
    return null;
  }

  let locked = $derived(invoice !== null && isLocked(invoice));
  let clientProjects = $derived(projects.filter((p) => p.clientId === draft.clientId));
  let discount = $derived<Discount>(
    draft.discountType === "percent"
      ? { type: "percent", value: Math.round((draft.discountValue ?? 0) * 100) }
      : draft.discountType === "fixed"
        ? { type: "fixed", value: draft.discountValue ?? 0 }
        : { type: "none" },
  );
  let totals = $derived(
    computeTotals(
      draft.items.map((i) => ({
        price: i.price ?? 0,
        quantityMilli: Math.round((i.quantity ?? 0) * 1000),
      })),
      discount,
      Math.round(draft.taxPercent * 100),
      invoice?.totals.paid.amountMinor ?? 0,
    ),
  );
  const money = (minor: number) => formatMoney(minor, draft.currency, i18n.locale);
  const error = (path: string) => (errors[path] ? codeMessage(errors[path]) : undefined);

  $effect(() => {
    if (invoice || draft.number.trim()) return;
    const issueDate = draft.issueDate;
    void call(commands.previewInvoiceNumber(issueDate))
      .then((n) => (numberPreview = n))
      .catch(() => (numberPreview = ""));
  });

  function pickClient(id: string) {
    draft.clientId = id || null;
    draft.projectId = null;
    const client = clients.find((c) => c.id === id);
    if (client?.defaultCurrency) draft.currency = client.defaultCurrency;
  }

  function pickProject(id: string) {
    draft.projectId = id || null;
    const project = projects.find((p) => p.id === id);
    if (project?.currency) draft.currency = project.currency;
  }

  function toInput(): InvoiceInput {
    return {
      number: draft.number,
      clientId: draft.clientId ?? "",
      projectId: draft.projectId,
      currency: draft.currency,
      issueDate: draft.issueDate,
      dueDate: draft.dueDate,
      status: draft.status,
      items: draft.items.map((i) => ({
        description: i.description,
        quantityMilli: Math.round((i.quantity ?? 0) * 1000),
        unit: i.unit,
        price: { amountMinor: i.price ?? 0, currency: draft.currency },
      })),
      discount,
      taxBp: Math.round(draft.taxPercent * 100),
      notesMd: draft.notesMd,
      language: draft.language,
      showSignature: draft.showSignature,
    };
  }

  async function save(event: SubmitEvent) {
    event.preventDefault();
    errors = draft.clientId ? {} : { clientId: "validation.required" };
    if (Object.keys(errors).length > 0) return;
    saving = true;
    try {
      const saved = invoice
        ? await call(commands.updateInvoice(invoice.id, toInput()))
        : await call(commands.createInvoice(toInput()));
      onsaved(saved);
    } catch (err) {
      const field = err instanceof AppError ? err.params.field : undefined;
      if (err instanceof AppError && typeof field === "string") errors = { [field]: err.code };
      else toast.show({ kind: "error", message: errorMessage(err) });
    } finally {
      saving = false;
    }
  }
</script>

<Dialog
  open
  title={invoice ? m.invoice_edit({ number: invoice.number }) : m.invoice_new()}
  {onclose}
>
  <form
    id="invoice-form"
    class="flex w-[48rem] max-w-full flex-col gap-4"
    novalidate
    onsubmit={save}
  >
    {#if locked}
      <p class="flex items-center gap-2 rounded-md bg-surface p-3 text-sm text-muted" role="status">
        <Lock class="size-4" aria-hidden="true" />
        {m.invoice_locked_hint()}
      </p>
    {/if}
    <fieldset disabled={locked} class="flex min-w-0 flex-col gap-4">
      <div class="grid gap-3 sm:grid-cols-2">
        <Select
          label={m.invoice_client()}
          required
          error={error("clientId")}
          options={clients.map((c) => ({ value: c.id, label: c.displayName }))}
          bind:value={() => draft.clientId ?? NONE, pickClient}
        />
        <Select
          label={m.invoice_project()}
          disabled={!draft.clientId}
          options={[
            { value: NONE, label: m.invoice_no_project() },
            ...clientProjects.map((p) => ({ value: p.id, label: p.name })),
          ]}
          bind:value={() => draft.projectId ?? NONE, pickProject}
        />
        <TextField
          label={m.invoice_number()}
          placeholder={numberPreview}
          hint={invoice ? undefined : m.invoice_number_hint()}
          error={error("number")}
          bind:value={draft.number}
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
          label={m.invoice_issue_date()}
          required
          error={error("issueDate")}
          bind:value={() => draft.issueDate, (v) => (draft.issueDate = v ?? draft.issueDate)}
        />
        <DatePicker
          label={m.invoice_due_date()}
          min={draft.issueDate}
          error={error("dueDate")}
          bind:value={draft.dueDate}
        />
        <Select
          label={m.invoice_status()}
          options={(["draft", "sent", "cancelled"] as const).map((s) => ({
            value: s,
            label: statusLabel(s),
          }))}
          bind:value={
            () => draft.status, (v) => (draft.status = (v ?? draft.status) as StoredStatus)
          }
        />
        <Select
          label={m.invoice_language()}
          options={[
            { value: "ru", label: "Русский" },
            { value: "en", label: "English" },
          ]}
          bind:value={() => draft.language, (v) => (draft.language = v ?? draft.language)}
        />
      </div>

      <section class="flex flex-col gap-2" aria-labelledby="invoice-items">
        <h3 id="invoice-items" class="text-sm font-medium text-fg">{m.invoice_items()}</h3>
        <ol class="flex flex-col gap-2">
          {#each draft.items as row, index (row.key)}
            <li
              class="grid grid-cols-[1fr_6rem_5rem_8rem_auto_auto] items-end gap-2"
              aria-label={m.invoice_item_n({ n: index + 1 })}
            >
              <TextField
                label={m.invoice_item_description()}
                error={error(`items.${index}.description`)}
                bind:value={row.description}
              />
              <NumberField
                label={m.invoice_item_quantity()}
                min={0}
                step={0.001}
                error={error(`items.${index}.quantity`)}
                bind:value={row.quantity}
              />
              <TextField label={m.invoice_item_unit()} bind:value={row.unit} />
              <MoneyField
                label={m.invoice_item_price()}
                currency={draft.currency}
                error={error(`items.${index}.price`)}
                bind:value={row.price}
              />
              <span class="pb-2 text-right text-sm tabular-nums text-fg" data-testid="line-amount"
                >{money(totals.lines[index] ?? 0)}</span
              >
              <Button
                variant="ghost"
                size="sm"
                aria-label={m.invoice_item_remove({ n: index + 1 })}
                onclick={() => draft.items.splice(index, 1)}
              >
                <Trash class="size-4" aria-hidden="true" />
              </Button>
            </li>
          {/each}
        </ol>
        <div>
          <Button size="sm" onclick={() => draft.items.push(item())}>
            <Plus class="size-4" aria-hidden="true" />
            {m.invoice_item_add()}
          </Button>
        </div>
      </section>

      <div class="grid gap-3 sm:grid-cols-3">
        <Select
          label={m.invoice_discount()}
          options={[
            { value: "none", label: m.invoice_discount_none() },
            { value: "percent", label: m.invoice_discount_percent() },
            { value: "fixed", label: m.invoice_discount_fixed() },
          ]}
          bind:value={
            () => draft.discountType,
            (v) => {
              draft.discountType = (v ?? "none") as Discount["type"];
              draft.discountValue = null;
            }
          }
        />
        {#if draft.discountType === "percent"}
          <NumberField
            label={m.invoice_discount_value_percent()}
            min={0}
            max={100}
            step={0.01}
            bind:value={draft.discountValue}
          />
        {:else if draft.discountType === "fixed"}
          <MoneyField
            label={m.invoice_discount_value_fixed()}
            currency={draft.currency}
            bind:value={draft.discountValue}
          />
        {:else}
          <span></span>
        {/if}
        <NumberField
          label={m.invoice_tax_percent()}
          min={0}
          max={100}
          step={0.01}
          bind:value={() => draft.taxPercent, (v) => (draft.taxPercent = v ?? 0)}
        />
      </div>

      <TextArea label={m.invoice_notes()} rows={3} bind:value={draft.notesMd} />
      <Select
        label={m.invoice_show_signature()}
        options={[
          { value: DEFAULT, label: m.invoice_show_signature_default() },
          { value: "yes", label: m.invoice_show_signature_yes() },
          { value: "no", label: m.invoice_show_signature_no() },
        ]}
        bind:value={
          () => (draft.showSignature === null ? DEFAULT : draft.showSignature ? "yes" : "no"),
          (v) => (draft.showSignature = v === "yes" ? true : v === "no" ? false : null)
        }
      />
    </fieldset>

    <dl class="ml-auto grid grid-cols-[auto_auto] gap-x-6 gap-y-1 text-sm tabular-nums">
      <dt class="text-muted">{m.invoice_subtotal()}</dt>
      <dd class="text-right">{money(totals.subtotal)}</dd>
      {#if totals.discount}
        <dt class="text-muted">{m.invoice_discount()}</dt>
        <dd class="text-right">−{money(totals.discount)}</dd>
      {/if}
      {#if totals.tax}
        <dt class="text-muted">{m.invoice_tax()}</dt>
        <dd class="text-right">{money(totals.tax)}</dd>
      {/if}
      <dt class="font-medium text-fg">{m.invoice_total()}</dt>
      <dd class="text-right font-medium text-fg" data-testid="invoice-total">
        {money(totals.total)}
      </dd>
      {#if totals.paid}
        <dt class="text-muted">{m.invoice_paid()}</dt>
        <dd class="text-right">{money(totals.paid)}</dd>
        <dt class="text-muted">{m.invoice_due()}</dt>
        <dd class="text-right">{money(totals.due)}</dd>
      {/if}
    </dl>
  </form>
  {#snippet footer()}
    <Button onclick={onclose}>{locked ? m.ui_close() : m.ui_cancel()}</Button>
    {#if !locked}
      <Button type="submit" form="invoice-form" variant="primary" loading={saving}
        >{m.ui_save()}</Button
      >
    {/if}
  {/snippet}
</Dialog>
