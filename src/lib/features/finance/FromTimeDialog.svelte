<script lang="ts">
  import {
    commands,
    type Client_Serialize as Client,
    type Grouping,
    type Invoice_Serialize as Invoice,
    type Project_Serialize as Project,
    type TimeEntry_Serialize as TimeEntry,
  } from "$lib/api/bindings";
  import { call } from "$lib/api/client";
  import { errorMessage } from "$lib/i18n/errors";
  import { i18n } from "$lib/i18n/locale.svelte";
  import { m } from "$lib/paraglide/messages";
  import Button from "$lib/ui/Button.svelte";
  import { addDays, longDate, todayIso } from "$lib/ui/date";
  import DatePicker from "$lib/ui/DatePicker.svelte";
  import DateRangePicker from "$lib/ui/DateRangePicker.svelte";
  import Dialog from "$lib/ui/Dialog.svelte";
  import RadioGroup from "$lib/ui/RadioGroup.svelte";
  import Select from "$lib/ui/Select.svelte";
  import Spinner from "$lib/ui/Spinner.svelte";
  import TextField from "$lib/ui/TextField.svelte";
  import { toast } from "$lib/ui/toast.svelte";
  import { localDate, midnight } from "../calendar/time";
  import { entryLabel, hoursMinutes } from "../time/format";

  let {
    clients,
    projects,
    today = todayIso(),
    oncreated,
    onclose,
  }: {
    clients: Client[];
    projects: Project[];
    today?: string;
    /** The new draft invoice, to open in the editor. */
    oncreated: (invoice: Invoice) => void;
    onclose: () => void;
  } = $props();

  const NONE = "";
  // `today` only seeds the form: the month so far, issued today.
  // svelte-ignore state_referenced_locally
  const seed = { today };
  let clientId = $state<string | null>(null);
  let projectId = $state<string | null>(null);
  let from = $state<string | null>(`${seed.today.slice(0, 8)}01`);
  let to = $state<string | null>(seed.today);
  let grouping = $state<Grouping>("by_task");
  let description = $state("");
  let issueDate = $state<string | null>(seed.today);
  let dueDate = $state<string | null>(null);
  let entries = $state<TimeEntry[] | null>(null);
  let creating = $state(false);

  let clientProjects = $derived(projects.filter((p) => p.clientId === clientId));
  let seconds = $derived(
    (entries ?? []).reduce((sum, e) => sum + ((e.endedAt ?? e.startedAt) - e.startedAt) / 1000, 0),
  );
  let span = $derived(from && to ? { from: midnight(from), to: midnight(addDays(to, 1)) } : null);

  $effect(() => {
    if (!clientId || !span) {
      entries = null;
      return;
    }
    const filter = { ...span, clientId, projectId, billable: true };
    void call(commands.unbilledTime(filter))
      .then((list) => (entries = list))
      .catch((err) => toast.show({ kind: "error", message: errorMessage(err) }));
  });

  async function create(event: SubmitEvent) {
    event.preventDefault();
    if (!clientId || !span || !issueDate) return;
    creating = true;
    try {
      const created = await call(
        commands.invoiceFromTime({
          clientId,
          projectId,
          ...span,
          grouping,
          description,
          number: "",
          issueDate,
          dueDate,
          taxBp: 0,
          language: i18n.locale === "ru" ? "ru" : "en",
        }),
      );
      oncreated(created);
    } catch (err) {
      toast.show({ kind: "error", message: errorMessage(err) });
    } finally {
      creating = false;
    }
  }
</script>

<Dialog open title={m.invoice_from_time_title()} {onclose}>
  <form id="from-time-form" class="flex w-[36rem] max-w-full flex-col gap-4" onsubmit={create}>
    <div class="grid gap-3 sm:grid-cols-2">
      <Select
        label={m.invoice_client()}
        required
        options={clients.map((c) => ({ value: c.id, label: c.displayName }))}
        bind:value={
          () => clientId ?? NONE,
          (v) => {
            clientId = v || null;
            projectId = null;
          }
        }
      />
      <Select
        label={m.invoice_project()}
        disabled={!clientId}
        options={[
          { value: NONE, label: m.invoice_no_project() },
          ...clientProjects.map((p) => ({ value: p.id, label: p.name })),
        ]}
        bind:value={() => projectId ?? NONE, (v) => (projectId = v || null)}
      />
    </div>
    <DateRangePicker
      startLabel={m.time_from()}
      endLabel={m.time_to()}
      bind:start={from}
      bind:end={to}
    />

    {#if clientId}
      <section class="flex flex-col gap-2" aria-labelledby="from-time-entries">
        <h3 id="from-time-entries" class="text-sm font-medium text-fg">
          {m.invoice_from_time_entries()}
        </h3>
        {#if entries === null}
          <Spinner />
        {:else if entries.length === 0}
          <p class="text-sm text-muted">{m.invoice_from_time_empty()}</p>
        {:else}
          <ul
            class="flex max-h-56 flex-col divide-y divide-border overflow-y-auto rounded-md border border-border"
            aria-labelledby="from-time-entries"
          >
            {#each entries as e (e.id)}
              <li class="flex items-center gap-3 px-3 py-1.5 text-sm">
                <span class="w-28 shrink-0 text-muted"
                  >{longDate(localDate(e.startedAt), i18n.locale)}</span
                >
                <span class="flex-1 truncate text-fg">{entryLabel(e)}</span>
                <span class="tabular-nums text-fg"
                  >{hoursMinutes(((e.endedAt ?? e.startedAt) - e.startedAt) / 1000)}</span
                >
              </li>
            {/each}
          </ul>
          <p class="text-right text-sm font-medium text-fg">
            {m.invoice_from_time_total({ duration: hoursMinutes(seconds) })}
          </p>
        {/if}
      </section>
    {/if}

    <RadioGroup
      label={m.invoice_grouping()}
      options={[
        { value: "by_task", label: m.invoice_grouping_by_task() },
        { value: "single_line", label: m.invoice_grouping_single() },
      ]}
      bind:value={() => grouping, (v) => (grouping = v as Grouping)}
    />
    {#if grouping === "single_line"}
      <TextField label={m.invoice_single_description()} bind:value={description} />
    {/if}
    <div class="grid gap-3 sm:grid-cols-2">
      <DatePicker label={m.invoice_issue_date()} required bind:value={issueDate} />
      <DatePicker label={m.invoice_due_date()} min={issueDate ?? undefined} bind:value={dueDate} />
    </div>
  </form>
  {#snippet footer()}
    <Button onclick={onclose}>{m.ui_cancel()}</Button>
    <Button
      type="submit"
      form="from-time-form"
      variant="primary"
      loading={creating}
      disabled={!entries?.length || !issueDate}>{m.invoice_from_time_create()}</Button
    >
  {/snippet}
</Dialog>
