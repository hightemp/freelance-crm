<script lang="ts">
  import { commands, type Invoice_Serialize as Invoice } from "$lib/api/bindings";
  import { call } from "$lib/api/client";
  import { errorMessage } from "$lib/i18n/errors";
  import { formatMoney } from "$lib/i18n/format";
  import { i18n } from "$lib/i18n/locale.svelte";
  import { m } from "$lib/paraglide/messages";
  import Button from "$lib/ui/Button.svelte";
  import { todayIso } from "$lib/ui/date";
  import DatePicker from "$lib/ui/DatePicker.svelte";
  import Dialog from "$lib/ui/Dialog.svelte";
  import MoneyField from "$lib/ui/MoneyField.svelte";
  import TextField from "$lib/ui/TextField.svelte";
  import { toast } from "$lib/ui/toast.svelte";

  let {
    invoice,
    today = todayIso(),
    onsaved,
    onclose,
  }: {
    invoice: Invoice;
    today?: string;
    onsaved: () => void;
    onclose: () => void;
  } = $props();

  // The invoice only seeds the form: what is still due, paid today.
  // svelte-ignore state_referenced_locally
  const seed = { due: Math.max(invoice.totals.due.amountMinor, 0), today };
  let amount = $state<number | null>(seed.due);
  let paidOn = $state<string | null>(seed.today);
  let method = $state("");
  let note = $state("");
  let saving = $state(false);

  async function save(event: SubmitEvent) {
    event.preventDefault();
    if (!amount || !paidOn) return;
    saving = true;
    try {
      const saved = await call(
        commands.createPayment({
          invoiceId: invoice.id,
          projectId: null,
          amount: { amountMinor: amount, currency: invoice.currency },
          paidOn,
          method,
          note,
        }),
      );
      if (saved.overpaid) {
        const extra = formatMoney(saved.overpaid.amountMinor, saved.overpaid.currency, i18n.locale);
        toast.show({ kind: "info", message: m.payment_overpaid({ amount: extra }) });
      } else {
        toast.show({ kind: "success", message: m.payment_saved() });
      }
      onsaved();
    } catch (err) {
      toast.show({ kind: "error", message: errorMessage(err) });
    } finally {
      saving = false;
    }
  }
</script>

<Dialog open title={m.payment_title({ number: invoice.number })} {onclose}>
  <form id="payment-form" class="flex w-[24rem] max-w-full flex-col gap-4" onsubmit={save}>
    <MoneyField
      label={m.payment_amount()}
      currency={invoice.currency}
      required
      bind:value={amount}
    />
    <DatePicker label={m.payment_date()} required bind:value={paidOn} />
    <TextField label={m.payment_method()} bind:value={method} />
    <TextField label={m.payment_note()} bind:value={note} />
  </form>
  {#snippet footer()}
    <Button onclick={onclose}>{m.ui_cancel()}</Button>
    <Button
      type="submit"
      form="payment-form"
      variant="primary"
      loading={saving}
      disabled={!amount || !paidOn}>{m.ui_save()}</Button
    >
  {/snippet}
</Dialog>
