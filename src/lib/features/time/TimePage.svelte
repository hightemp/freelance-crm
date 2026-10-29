<script lang="ts">
  import Download from "@lucide/svelte/icons/download";
  import Plus from "@lucide/svelte/icons/plus";
  import { save } from "@tauri-apps/plugin-dialog";
  import {
    commands,
    type Client_Serialize as Client,
    type GroupBy,
    type Project_Serialize as Project,
    type TimeEntry_Serialize as TimeEntry,
    type TimeEntryInput,
    type TimeFilter,
    type TimeReport,
  } from "$lib/api/bindings";
  import { call } from "$lib/api/client";
  import { errorMessage } from "$lib/i18n/errors";
  import { formatMoney } from "$lib/i18n/format";
  import { i18n } from "$lib/i18n/locale.svelte";
  import { m } from "$lib/paraglide/messages";
  import Button from "$lib/ui/Button.svelte";
  import { addDays, longDate, todayIso } from "$lib/ui/date";
  import DateRangePicker from "$lib/ui/DateRangePicker.svelte";
  import { confirm } from "$lib/ui/dialogs.svelte";
  import Select from "$lib/ui/Select.svelte";
  import Spinner from "$lib/ui/Spinner.svelte";
  import Tabs from "$lib/ui/Tabs.svelte";
  import { toast } from "$lib/ui/toast.svelte";
  import { localDate, midnight } from "../calendar/time";
  import EntryDialog from "./EntryDialog.svelte";
  import EntryRow from "./EntryRow.svelte";
  import { hoursMinutes } from "./format";
  import { timer } from "./timer.svelte";

  let { today = todayIso() }: { today?: string } = $props();

  const ALL = "";
  // `today` only seeds the period.
  // svelte-ignore state_referenced_locally
  const start = { today };
  let from = $state<string | null>(`${start.today.slice(0, 8)}01`);
  let to = $state<string | null>(start.today);
  let clientId = $state<string | null>(null);
  let projectId = $state<string | null>(null);
  let billable = $state<"" | "yes" | "no">(ALL);
  let group = $state<GroupBy>("day");
  let tab = $state("journal");
  let entries = $state<TimeEntry[] | null>(null);
  let report = $state<TimeReport | null>(null);
  let clients = $state<Client[]>([]);
  let projects = $state<Project[]>([]);
  let adding = $state(false);

  const showError = (err: unknown) => toast.show({ kind: "error", message: errorMessage(err) });

  let filter = $derived<TimeFilter>({
    from: from ? midnight(from) : null,
    to: to ? midnight(addDays(to, 1)) : null,
    clientId,
    projectId,
    billable: billable === "" ? null : billable === "yes",
  });

  async function load() {
    try {
      const f = { ...filter };
      [entries, report] = await Promise.all([
        call(commands.listTimeEntries(f)),
        call(commands.timeReport(f, group)),
      ]);
    } catch (err) {
      showError(err);
    }
  }

  $effect(() => {
    void [filter, group, timer.running?.id];
    void load();
  });

  $effect(() => {
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

  /** Saves and reports overlaps; false if it failed. */
  async function persist(action: () => Promise<{ overlaps: string[] }>): Promise<boolean> {
    try {
      const { overlaps } = await action();
      if (overlaps.length > 0) {
        toast.show({
          message:
            overlaps.length === 1
              ? m.time_overlap_one()
              : m.time_overlap_many({ count: overlaps.length }),
        });
      }
      await load();
      return true;
    } catch (err) {
      showError(err);
      return false;
    }
  }

  async function remove(entry: TimeEntry) {
    const ok = await confirm({
      title: m.time_delete_title(),
      confirmLabel: m.ui_delete(),
      danger: true,
    });
    if (!ok) return;
    try {
      await call(commands.deleteTimeEntry(entry.id));
      await load();
    } catch (err) {
      showError(err);
    }
  }

  async function exportCsv() {
    const path = await save({
      defaultPath: "time.csv",
      filters: [{ name: "CSV", extensions: ["csv"] }],
    }).catch(() => null);
    if (!path) return;
    const headers = [
      m.csv_date(),
      m.csv_start(),
      m.csv_end(),
      m.csv_hours(),
      m.csv_client(),
      m.csv_project(),
      m.csv_task(),
      m.csv_description(),
      m.csv_billable(),
      m.csv_amount(),
      m.csv_currency(),
    ];
    try {
      await call(commands.exportTimeCsv(path, { ...filter }, headers));
      toast.show({ message: m.time_exported_toast() });
    } catch (err) {
      showError(err);
    }
  }

  // Journal grouped by local day, newest first.
  let days = $derived.by(() => {
    const out: { date: string; items: TimeEntry[] }[] = [];
    for (const e of entries ?? []) {
      const date = localDate(e.startedAt);
      if (out.at(-1)?.date !== date) out.push({ date, items: [] });
      out.at(-1)!.items.push(e);
    }
    return out;
  });

  const amounts = (list: { amountMinor: number; currency: string }[]) =>
    list.map((a) => formatMoney(a.amountMinor, a.currency, i18n.locale)).join(", ");

  function groupLabel(row: { key: string; label: string }): string {
    if (!row.key) return m.time_none();
    return group === "day" ? longDate(row.key, i18n.locale) : row.label;
  }

  let clientOptions = $derived([
    { value: ALL, label: m.time_all_clients() },
    ...clients.map((c) => ({ value: c.id, label: c.displayName })),
  ]);
  let projectOptions = $derived([
    { value: ALL, label: m.time_all_projects() },
    ...projects
      .filter((p) => !clientId || p.clientId === clientId)
      .map((p) => ({ value: p.id, label: p.name })),
  ]);
</script>

<div class="flex flex-col gap-4">
  <div class="grid items-end gap-3 md:grid-cols-[2fr_1fr_1fr_1fr]">
    <DateRangePicker
      startLabel={m.time_from()}
      endLabel={m.time_to()}
      bind:start={from}
      bind:end={to}
    />
    <Select
      label={m.time_client()}
      options={clientOptions}
      bind:value={
        () => clientId ?? ALL,
        (v) => {
          clientId = v || null;
          if (projectId && !projects.some((p) => p.id === projectId && p.clientId === clientId))
            projectId = null;
        }
      }
    />
    <Select
      label={m.time_project()}
      options={projectOptions}
      bind:value={() => projectId ?? ALL, (v) => (projectId = v || null)}
    />
    <Select
      label={m.time_billable()}
      options={[
        { value: ALL, label: m.time_billable_all() },
        { value: "yes", label: m.time_billable_yes() },
        { value: "no", label: m.time_billable_no() },
      ]}
      bind:value={() => billable, (v) => (billable = (v ?? ALL) as typeof billable)}
    />
  </div>

  <div class="flex flex-wrap items-center gap-3">
    {#if report}
      <span class="font-semibold text-fg"
        >{m.time_total({ hours: hoursMinutes(report.total.seconds) })}</span
      >
      {#if report.total.amounts.length > 0}<span class="text-muted"
          >{amounts(report.total.amounts)}</span
        >{/if}
    {/if}
    <span class="flex-1"></span>
    <Button size="sm" onclick={exportCsv}>
      <Download class="size-4" aria-hidden="true" />
      {m.time_export()}
    </Button>
    <Button size="sm" variant="primary" onclick={() => (adding = true)}>
      <Plus class="size-4" aria-hidden="true" />
      {m.time_add()}
    </Button>
  </div>

  <Tabs
    label={m.time_tabs()}
    tabs={[
      { id: "journal", label: m.time_journal() },
      { id: "report", label: m.time_report() },
    ]}
    bind:value={tab}
  >
    {#snippet panel(current)}
      {#if current === "journal"}
        {#if entries === null}
          <div class="flex justify-center py-4"><Spinner /></div>
        {:else if entries.length === 0}
          <p class="py-4 text-sm text-muted">{m.time_empty()}</p>
        {:else}
          <div class="flex flex-col gap-4 py-4">
            <ul aria-label={m.time_entries()} class="flex flex-col">
              {#each days as day (day.date)}
                {#each day.items as entry, i (entry.id)}
                  <EntryRow
                    {entry}
                    day={i === 0 ? longDate(day.date, i18n.locale) : undefined}
                    onsave={(input) =>
                      persist(() => call(commands.updateTimeEntry(entry.id, input)))}
                    ondelete={() => remove(entry)}
                  />
                {/each}
              {/each}
            </ul>
          </div>
        {/if}
      {:else}
        <div class="flex flex-col gap-3 py-4">
          <div class="max-w-xs">
            <Select
              label={m.time_group_by()}
              options={[
                { value: "day", label: m.time_group_day() },
                { value: "project", label: m.time_group_project() },
                { value: "task", label: m.time_group_task() },
              ]}
              bind:value={() => group, (v) => (group = (v ?? "day") as GroupBy)}
            />
          </div>
          {#if report}
            <table class="w-full text-sm">
              <thead class="text-left text-xs text-muted">
                <tr>
                  <th class="py-1 font-medium">{m.time_col_group()}</th>
                  <th class="py-1 text-right font-medium">{m.time_col_hours()}</th>
                  <th class="py-1 text-right font-medium">{m.time_col_amount()}</th>
                </tr>
              </thead>
              <tbody>
                {#each report.rows as row (row.key)}
                  <tr class="border-t border-border">
                    <td class="py-1">{groupLabel(row)}</td>
                    <td class="py-1 text-right tabular-nums">{hoursMinutes(row.seconds)}</td>
                    <td class="py-1 text-right tabular-nums">{amounts(row.amounts)}</td>
                  </tr>
                {/each}
              </tbody>
            </table>
          {/if}
        </div>
      {/if}
    {/snippet}
  </Tabs>
</div>

{#if adding}
  <EntryDialog
    date={today}
    {projects}
    onsave={(input: TimeEntryInput) => persist(() => call(commands.createTimeEntry(input)))}
    onclose={() => (adding = false)}
  />
{/if}
