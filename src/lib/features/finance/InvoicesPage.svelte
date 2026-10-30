<script lang="ts">
  import Clock from "@lucide/svelte/icons/clock";
  import Ellipsis from "@lucide/svelte/icons/ellipsis";
  import Plus from "@lucide/svelte/icons/plus";
  import { save } from "@tauri-apps/plugin-dialog";
  import {
    commands,
    type Client_Serialize as Client,
    type Invoice_Serialize as Invoice,
    type Project_Serialize as Project,
  } from "$lib/api/bindings";
  import { call } from "$lib/api/client";
  import { errorMessage } from "$lib/i18n/errors";
  import { formatIsoDate, formatMoney } from "$lib/i18n/format";
  import { i18n } from "$lib/i18n/locale.svelte";
  import { m } from "$lib/paraglide/messages";
  import Button from "$lib/ui/Button.svelte";
  import { todayIso } from "$lib/ui/date";
  import { confirm } from "$lib/ui/dialogs.svelte";
  import DropdownMenu from "$lib/ui/DropdownMenu.svelte";
  import Spinner from "$lib/ui/Spinner.svelte";
  import { toast } from "$lib/ui/toast.svelte";
  import FromTimeDialog from "./FromTimeDialog.svelte";
  import { statusLabel } from "./invoice";
  import InvoiceEditor from "./InvoiceEditor.svelte";
  import PaymentDialog from "./PaymentDialog.svelte";

  let {
    clientId = null,
    openId = null,
    today = todayIso(),
  }: {
    clientId?: string | null;
    /** An invoice to open in the editor once loaded (from the calendar). */
    openId?: string | null;
    today?: string;
  } = $props();

  let invoices = $state<Invoice[] | null>(null);
  let clients = $state<Client[]>([]);
  let projects = $state<Project[]>([]);
  /** The invoice in the editor: `null` for a new one. */
  let editing = $state<{ invoice: Invoice | null } | null>(null);
  let paying = $state<Invoice | null>(null);
  let fromTime = $state(false);
  // svelte-ignore state_referenced_locally
  let pending = openId;

  const showError = (err: unknown) => toast.show({ kind: "error", message: errorMessage(err) });
  const money = (i: Invoice, minor: number) => formatMoney(minor, i.currency, i18n.locale);

  const STATUS_TONE: Record<Invoice["status"], string> = {
    draft: "border border-border text-muted",
    sent: "bg-accent/10 text-accent",
    partially_paid: "bg-accent/20 text-fg",
    paid: "bg-accent text-accent-fg",
    overdue: "bg-danger/15 text-danger",
    cancelled: "border border-border text-muted line-through",
  };

  async function load() {
    try {
      invoices = await call(commands.listInvoices(clientId, null));
      if (pending) {
        const found = invoices.find((i) => i.id === pending);
        pending = null;
        if (found) editing = { invoice: found };
      }
    } catch (err) {
      showError(err);
    }
  }

  $effect(() => {
    void load();
    void Promise.all([
      call(commands.listClients({ query: null, status: null, archived: false, tagIds: [] })),
      call(commands.listProjects({ query: null, status: null, clientId: null, archived: false })),
    ])
      .then(([c, p]) => {
        clients = c;
        projects = p;
      })
      .catch(showError);
  });

  function actionsFor(i: Invoice) {
    const open = i.status !== "draft" && i.status !== "cancelled" && i.status !== "paid";
    return [
      { id: "open", label: m.invoice_open() },
      { id: "pay", label: m.payment_add(), disabled: !open },
      { separator: true as const },
      { id: "preview", label: m.invoice_preview_pdf() },
      { id: "save", label: m.invoice_save_pdf() },
      { separator: true as const },
      { id: "delete", label: m.ui_delete(), danger: true },
    ];
  }

  async function onAction(i: Invoice, action: string) {
    try {
      switch (action) {
        case "open":
          editing = { invoice: i };
          break;
        case "pay":
          paying = i;
          break;
        case "preview":
          await call(commands.openInvoicePdf(i.id));
          break;
        case "save": {
          const path = await save({
            defaultPath: `${i.number}.pdf`,
            filters: [{ name: "PDF", extensions: ["pdf"] }],
          });
          if (!path) return;
          await call(commands.saveInvoicePdf(i.id, path));
          toast.show({ kind: "success", message: m.invoice_pdf_saved() });
          break;
        }
        case "delete": {
          const ok = await confirm({
            title: m.invoice_delete_title({ number: i.number }),
            message: m.invoice_delete_body(),
            confirmLabel: m.ui_delete(),
            danger: true,
          });
          if (!ok) return;
          await call(commands.deleteInvoice(i.id));
          await load();
          break;
        }
      }
    } catch (err) {
      showError(err);
    }
  }
</script>

<section class="flex flex-col gap-4">
  <div class="flex gap-2">
    <Button variant="primary" onclick={() => (editing = { invoice: null })}>
      <Plus class="size-4" aria-hidden="true" />
      {m.invoice_new()}
    </Button>
    <Button onclick={() => (fromTime = true)}>
      <Clock class="size-4" aria-hidden="true" />
      {m.invoice_from_time()}
    </Button>
  </div>

  {#if invoices === null}
    <Spinner />
  {:else if invoices.length === 0}
    <p class="text-sm text-muted">{m.invoice_list_empty()}</p>
  {:else}
    <table class="w-full text-sm">
      <thead class="text-left text-muted">
        <tr>
          <th class="py-2 font-medium">{m.invoice_col_number()}</th>
          <th class="py-2 font-medium">{m.invoice_col_client()}</th>
          <th class="py-2 font-medium">{m.invoice_col_issued()}</th>
          <th class="py-2 font-medium">{m.invoice_col_due()}</th>
          <th class="py-2 text-right font-medium">{m.invoice_col_total()}</th>
          <th class="py-2 font-medium">{m.invoice_col_status()}</th>
          <th class="py-2"><span class="sr-only">{m.invoice_col_actions()}</span></th>
        </tr>
      </thead>
      <tbody class="divide-y divide-border">
        {#each invoices as i (i.id)}
          <tr>
            <td class="py-2">
              <button
                type="button"
                class="font-medium text-fg hover:underline"
                onclick={() => (editing = { invoice: i })}>{i.number}</button
              >
            </td>
            <td class="py-2 text-fg">
              {i.clientName}
              {#if i.projectName}<span class="text-muted"> · {i.projectName}</span>{/if}
            </td>
            <td class="py-2 text-muted">{formatIsoDate(i.issueDate, i18n.locale)}</td>
            <td class="py-2 text-muted">
              {i.dueDate ? formatIsoDate(i.dueDate, i18n.locale) : "—"}
            </td>
            <td class="py-2 text-right tabular-nums text-fg"
              >{money(i, i.totals.total.amountMinor)}</td
            >
            <td class="py-2">
              <span class="rounded-full px-2 py-0.5 text-xs {STATUS_TONE[i.status]}"
                >{statusLabel(i.status)}</span
              >
            </td>
            <td class="py-2 text-right">
              <DropdownMenu
                items={actionsFor(i)}
                onSelect={(id) => void onAction(i, id)}
                placement="bottom-end"
              >
                {#snippet trigger(props)}
                  <Button
                    {...props}
                    variant="ghost"
                    size="sm"
                    aria-label={m.invoice_actions({ number: i.number })}
                  >
                    <Ellipsis class="size-4" aria-hidden="true" />
                  </Button>
                {/snippet}
              </DropdownMenu>
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</section>

{#if editing}
  <InvoiceEditor
    invoice={editing.invoice}
    {clients}
    {projects}
    {today}
    onsaved={() => {
      editing = null;
      void load();
    }}
    onclose={() => (editing = null)}
  />
{/if}
{#if fromTime}
  <FromTimeDialog
    {clients}
    {projects}
    {today}
    oncreated={(created) => {
      fromTime = false;
      editing = { invoice: created };
      void load();
    }}
    onclose={() => (fromTime = false)}
  />
{/if}
{#if paying}
  <PaymentDialog
    invoice={paying}
    {today}
    onsaved={() => {
      paying = null;
      void load();
    }}
    onclose={() => (paying = null)}
  />
{/if}
