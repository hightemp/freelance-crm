<script lang="ts">
  import Lock from "@lucide/svelte/icons/lock";
  import Pencil from "@lucide/svelte/icons/pencil";
  import Trash2 from "@lucide/svelte/icons/trash-2";
  import type { TimeEntry_Serialize as TimeEntry, TimeEntryInput } from "$lib/api/bindings";
  import { i18n } from "$lib/i18n/locale.svelte";
  import { m } from "$lib/paraglide/messages";
  import Button from "$lib/ui/Button.svelte";
  import Checkbox from "$lib/ui/Checkbox.svelte";
  import DateTimePicker from "$lib/ui/DateTimePicker.svelte";
  import TextField from "$lib/ui/TextField.svelte";
  import { clock } from "../calendar/format";
  import { localDateTime } from "../calendar/time";
  import { entryLabel, fromLocal, hoursMinutes } from "./format";

  let {
    entry,
    day,
    onsave,
    ondelete,
  }: {
    entry: TimeEntry;
    /** Shown above the first entry of a day. */
    day?: string;
    /** Resolves when saved; the row then leaves edit mode. */
    onsave: (input: TimeEntryInput) => Promise<boolean>;
    ondelete: () => void;
  } = $props();

  let editing = $state(false);
  let draft = $state({ description: "", start: "", end: "", billable: true });
  let saving = $state(false);
  let label = $derived(entryLabel(entry));
  let seconds = $derived(((entry.endedAt ?? entry.startedAt) - entry.startedAt) / 1000);

  function edit() {
    draft = {
      description: entry.description,
      start: localDateTime(entry.startedAt),
      end: localDateTime(entry.endedAt ?? entry.startedAt),
      billable: entry.billable,
    };
    editing = true;
  }

  async function save(event: SubmitEvent) {
    event.preventDefault();
    saving = true;
    const saved = await onsave({
      target: { taskId: entry.taskId, projectId: entry.projectId, clientId: entry.clientId },
      description: draft.description,
      startedAt: fromLocal(draft.start),
      endedAt: fromLocal(draft.end),
      billable: draft.billable,
    });
    saving = false;
    if (saved) editing = false;
  }
</script>

<li class="px-3 py-2">
  {#if day}
    <div class="-mx-3 mb-1 px-3 pt-2 text-xs font-semibold text-muted">{day}</div>
  {/if}
  {#if editing}
    <form class="flex flex-col gap-3" onsubmit={save}>
      <TextField label={m.time_description()} bind:value={draft.description} />
      <div class="grid gap-3 sm:grid-cols-2">
        <DateTimePicker
          dateLabel={m.time_start_date()}
          timeLabel={m.time_start_time()}
          bind:value={() => draft.start, (v) => (draft.start = v ?? draft.start)}
        />
        <DateTimePicker
          dateLabel={m.time_end_date()}
          timeLabel={m.time_end_time()}
          bind:value={() => draft.end, (v) => (draft.end = v ?? draft.end)}
        />
      </div>
      <Checkbox label={m.time_billable()} bind:checked={draft.billable} />
      <div class="flex gap-2">
        <Button type="submit" variant="primary" loading={saving}>{m.ui_save()}</Button>
        <Button onclick={() => (editing = false)}>{m.ui_cancel()}</Button>
      </div>
    </form>
  {:else}
    <div class="flex items-center gap-3 text-sm">
      <span class="w-32 shrink-0 tabular-nums text-muted">
        {clock(entry.startedAt, i18n.locale)}–{clock(entry.endedAt ?? entry.startedAt, i18n.locale)}
      </span>
      <span class="min-w-0 flex-1 truncate text-fg">{label}</span>
      {#if !entry.billable}<span class="text-xs text-muted">{m.time_billable_no()}</span>{/if}
      <span class="w-14 shrink-0 text-right font-medium tabular-nums">{hoursMinutes(seconds)}</span>
      {#if entry.invoiceId}
        <span class="flex w-20 shrink-0 items-center gap-1 text-xs text-muted">
          <Lock class="size-3.5" aria-hidden="true" />{m.time_invoiced()}
        </span>
      {:else}
        <span class="flex w-20 shrink-0 justify-end gap-1">
          <Button variant="ghost" size="sm" aria-label={m.time_edit({ label })} onclick={edit}>
            <Pencil class="size-4" aria-hidden="true" />
          </Button>
          <Button
            variant="ghost"
            size="sm"
            aria-label={m.time_delete({ label })}
            onclick={ondelete}
          >
            <Trash2 class="size-4" aria-hidden="true" />
          </Button>
        </span>
      {/if}
    </div>
  {/if}
</li>
