<script lang="ts">
  import X from "@lucide/svelte/icons/x";
  import {
    commands,
    type EditScope,
    type EventInput,
    type Event_Serialize as Event,
    type Client_Serialize as Client,
    type Project_Serialize as Project,
  } from "$lib/api/bindings";
  import { AppError, call } from "$lib/api/client";
  import { codeMessage, errorMessage } from "$lib/i18n/errors";
  import { i18n } from "$lib/i18n/locale.svelte";
  import { m } from "$lib/paraglide/messages";
  import Button from "$lib/ui/Button.svelte";
  import Combobox from "$lib/ui/Combobox.svelte";
  import DatePicker from "$lib/ui/DatePicker.svelte";
  import DateTimePicker from "$lib/ui/DateTimePicker.svelte";
  import Dialog from "$lib/ui/Dialog.svelte";
  import { confirm } from "$lib/ui/dialogs.svelte";
  import NumberField from "$lib/ui/NumberField.svelte";
  import Select from "$lib/ui/Select.svelte";
  import Switch from "$lib/ui/Switch.svelte";
  import TextArea from "$lib/ui/TextArea.svelte";
  import TextField from "$lib/ui/TextField.svelte";
  import { toast } from "$lib/ui/toast.svelte";
  import { buildRule, readRule, type Ends, type Preset } from "./repeat";
  import ScopeDialog from "./ScopeDialog.svelte";
  import { reminderLabel, REMINDER_PRESETS, shiftLocal, systemTimeZone } from "./labels";

  interface Occurrence {
    recurrenceId: string | null;
    start: string;
    end: string;
  }

  let {
    event = null,
    occurrence = null,
    initial,
    timeZone = systemTimeZone(),
    create,
    onsaved,
    onclose,
  }: {
    /** The event to edit (a series, an edited occurrence or a single one). */
    event?: Event | null;
    /** Which occurrence of a series was opened. */
    occurrence?: Occurrence | null;
    /** Timing of a new event. */
    initial?: { start: string; end: string; allDay: boolean };
    /** Zone of new events. */
    timeZone?: string;
    /** Creates a new event instead of `create_event` (e.g. a follow-up). */
    create?: (input: EventInput) => Promise<{ id: string }>;
    onsaved: () => void;
    onclose: () => void;
  } = $props();

  const NONE = "";
  // Read once: the form edits its own copy.
  // svelte-ignore state_referenced_locally
  const seed = { event, occurrence, initial, timeZone };
  /** Series this occurrence belongs to, if any. */
  const seriesId = seed.event?.seriesId ?? (seed.event?.rrule ? seed.event.id : null);
  const recurrenceId = seed.occurrence?.recurrenceId ?? seed.event?.recurrenceId ?? null;
  const inSeries = seriesId !== null && recurrenceId !== null;

  const startText = seed.occurrence?.start ?? seed.event?.start ?? seed.initial?.start ?? "";
  const endText = seed.occurrence?.end ?? seed.event?.end ?? seed.initial?.end ?? "";
  const allDayStart = seed.event?.allDay ?? seed.initial?.allDay ?? false;
  const repeatSeed = readRule(
    seed.event?.rrule ?? null,
    (seed.event?.start ?? startText).slice(0, 10),
  );

  let draft = $state({
    title: seed.event?.title ?? "",
    descriptionMd: seed.event?.descriptionMd ?? "",
    location: seed.event?.location ?? "",
    allDay: allDayStart,
    start: startText,
    end: endText,
    tzid: seed.event?.tzid ?? seed.timeZone,
    clientId: seed.event?.clientId ?? null,
    projectId: seed.event?.projectId ?? null,
  });
  let preset = $state<Preset>(repeatSeed.preset);
  let customRule = $state(repeatSeed.rule ?? "");
  let endsKind = $state<Ends["kind"]>(repeatSeed.ends.kind);
  let count = $state<number | null>(repeatSeed.ends.kind === "count" ? repeatSeed.ends.count : 10);
  let until = $state<string | null>(repeatSeed.ends.kind === "until" ? repeatSeed.ends.date : null);
  let reminders = $state<number[]>([]);
  let errors = $state<Record<string, string>>({});
  let saving = $state(false);
  let clients = $state<Client[]>([]);
  let projects = $state<Project[]>([]);
  /** A pending scope question: for saving or deleting. */
  let asking = $state<"save" | "delete" | null>(null);

  const error = (field: string) => (errors[field] ? codeMessage(errors[field]) : undefined);
  let startDate = $derived(draft.start.slice(0, 10));

  $effect(() => {
    void Promise.all([
      call(commands.listClients({ query: null, status: null, archived: false, tagIds: [] })),
      call(commands.listProjects({ query: null, status: null, clientId: null, archived: false })),
    ]).then(([cls, prs]) => {
      clients = cls;
      projects = prs;
    });
    const owner = seriesId ?? seed.event?.id;
    if (owner) {
      void call(commands.listReminders({ type: "event", id: owner })).then(
        (list) => (reminders = list.map((r) => r.minutesBefore)),
      );
    }
  });

  /** All-day switches keep the dates and pick a default time back. */
  function setAllDay(on: boolean) {
    draft.allDay = on;
    if (on) {
      draft.start = draft.start.slice(0, 10);
      draft.end = draft.end.slice(0, 10);
    } else {
      draft.start = `${draft.start.slice(0, 10)}T09:00`;
      draft.end = `${draft.end.slice(0, 10)}T10:00`;
    }
  }

  let project = $derived(projects.find((p) => p.id === draft.projectId));
  let clientOptions = $derived(
    project
      ? [{ value: project.clientId, label: project.clientName }]
      : [
          { value: NONE, label: m.event_no_client() },
          ...clients.map((c) => ({ value: c.id, label: c.displayName })),
        ],
  );
  let projectOptions = $derived([
    { value: NONE, label: m.event_no_project() },
    ...projects
      .filter((p) => !draft.clientId || p.clientId === draft.clientId)
      .map((p) => ({ value: p.id, label: p.name })),
  ]);

  let repeatOptions = $derived.by(() => {
    const date = new Date(`${startDate}T00:00:00Z`);
    const day = new Intl.DateTimeFormat(i18n.locale, { weekday: "long", timeZone: "UTC" }).format(
      date,
    );
    const dayMonth = new Intl.DateTimeFormat(i18n.locale, {
      day: "numeric",
      month: "long",
      timeZone: "UTC",
    }).format(date);
    return [
      { value: "none", label: m.repeat_none() },
      { value: "daily", label: m.repeat_daily() },
      { value: "weekdays", label: m.repeat_weekdays() },
      { value: "weekly", label: m.repeat_weekly({ day }) },
      { value: "monthly", label: m.repeat_monthly({ day: date.getUTCDate() }) },
      { value: "yearly", label: m.repeat_yearly({ date: dayMonth }) },
      { value: "custom", label: m.repeat_custom() },
    ];
  });
  const endsOptions = [
    { value: "never", label: m.ends_never() },
    { value: "count", label: m.ends_count() },
    { value: "until", label: m.ends_until() },
  ];
  const zoneOptions = (Intl.supportedValuesOf?.("timeZone") ?? [seed.timeZone]).map((z) => ({
    value: z,
    label: z.replaceAll("_", " "),
  }));
  let reminderOptions = $derived(
    REMINDER_PRESETS.filter((r) => !reminders.includes(r)).map((r) => ({
      value: String(r),
      label: reminderLabel(r),
    })),
  );

  function ends(): Ends {
    if (endsKind === "count") return { kind: "count", count: Math.max(1, count ?? 1) };
    if (endsKind === "until" && until) return { kind: "until", date: until };
    return { kind: "never" };
  }

  function input(): EventInput {
    return {
      title: draft.title.trim(),
      descriptionMd: draft.descriptionMd,
      location: draft.location.trim() || null,
      allDay: draft.allDay,
      start: draft.start,
      end: draft.end,
      tzid: draft.tzid,
      clientId: draft.clientId,
      projectId: draft.projectId,
      taskId: seed.event?.taskId ?? null,
      contactId: seed.event?.contactId ?? null,
      rrule: buildRule({ preset, rule: customRule, ends: ends() }, startDate),
    };
  }

  function showError(err: unknown) {
    const field = err instanceof AppError ? err.params.field : undefined;
    if (err instanceof AppError && typeof field === "string") errors = { [field]: err.code };
    else if (err instanceof AppError && err.code.startsWith("rrule.")) errors = { rrule: err.code };
    else toast.show({ kind: "error", message: errorMessage(err) });
  }

  async function run(action: () => Promise<string | null>) {
    saving = true;
    try {
      const owner = await action();
      if (owner) await call(commands.setReminders({ type: "event", id: owner }, reminders));
      onsaved();
    } catch (err) {
      showError(err);
    } finally {
      saving = false;
    }
  }

  function submit(e: SubmitEvent) {
    e.preventDefault();
    errors = draft.title.trim() === "" ? { title: "validation.required" } : {};
    if (Object.keys(errors).length > 0) return;
    if (inSeries) {
      asking = "save";
      return;
    }
    void run(async () => {
      const saved = seed.event
        ? await call(commands.updateEvent(seed.event.id, input()))
        : create
          ? await create(input())
          : await call(commands.createEvent(input()));
      return saved.id;
    });
  }

  function saveIn(scope: EditScope) {
    asking = null;
    const data = input();
    if (scope === "this") data.rrule = null;
    if (scope === "all" && seed.occurrence && seed.event) {
      // Move the series by however much this occurrence was moved.
      const base = seed.event.seriesId ? null : seed.event;
      if (base) {
        data.start = shiftLocal(base.start, seed.occurrence.start, draft.start);
        data.end = shiftLocal(base.end, seed.occurrence.end, draft.end);
      }
    }
    void run(async () => {
      await call(commands.updateOccurrence(seriesId!, recurrenceId!, scope, data));
      return seriesId;
    });
  }

  async function remove() {
    if (inSeries) {
      asking = "delete";
      return;
    }
    const ok = await confirm({
      title: m.event_delete_title(),
      confirmLabel: m.ui_delete(),
      danger: true,
    });
    if (ok) void run(async () => (await call(commands.deleteEvent(seed.event!.id)), null));
  }

  function deleteIn(scope: EditScope) {
    asking = null;
    void run(
      async () => (await call(commands.deleteOccurrence(seriesId!, recurrenceId!, scope)), null),
    );
  }
</script>

{#if asking}
  <ScopeDialog
    title={asking === "save" ? m.scope_edit_title() : m.scope_delete_title()}
    onchoose={asking === "save" ? saveIn : deleteIn}
    oncancel={() => (asking = null)}
  />
{:else}
  <Dialog open title={seed.event ? m.event_edit_title() : m.event_new_title()} {onclose}>
    <form
      id="event-form"
      class="flex w-[40rem] max-w-full flex-col gap-4"
      novalidate
      onsubmit={submit}
    >
      <TextField label={m.event_title()} required bind:value={draft.title} error={error("title")} />
      <Switch label={m.event_all_day()} bind:checked={() => draft.allDay, setAllDay} />
      {#if draft.allDay}
        <div class="grid gap-4 sm:grid-cols-2">
          <DatePicker
            label={m.event_start_date()}
            bind:value={() => draft.start, (v) => (draft.start = v ?? draft.start)}
            error={error("start")}
          />
          <DatePicker
            label={m.event_end_date()}
            bind:value={() => draft.end, (v) => (draft.end = v ?? draft.end)}
            error={error("end")}
          />
        </div>
      {:else}
        <DateTimePicker
          dateLabel={m.event_start_date()}
          timeLabel={m.event_start_time()}
          bind:value={() => draft.start, (v) => (draft.start = v ?? draft.start)}
          error={error("start")}
        />
        <DateTimePicker
          dateLabel={m.event_end_date()}
          timeLabel={m.event_end_time()}
          bind:value={() => draft.end, (v) => (draft.end = v ?? draft.end)}
          error={error("end")}
        />
        <Combobox
          label={m.event_timezone()}
          options={zoneOptions}
          bind:value={() => draft.tzid, (v) => (draft.tzid = v ?? draft.tzid)}
        />
        {#if error("tzid")}<p class="text-sm text-danger">{error("tzid")}</p>{/if}
      {/if}
      <div class="grid gap-4 sm:grid-cols-2">
        <Select
          label={m.event_repeat()}
          options={repeatOptions}
          bind:value={() => preset, (v) => (preset = (v ?? "none") as Preset)}
        />
        {#if preset !== "none" && preset !== "custom"}
          <Select
            label={m.event_ends()}
            options={endsOptions}
            bind:value={() => endsKind, (v) => (endsKind = (v ?? "never") as Ends["kind"])}
          />
        {/if}
      </div>
      {#if preset === "custom"}
        <TextField label={m.event_rrule()} bind:value={customRule} error={error("rrule")} />
      {:else if error("rrule")}
        <p class="text-sm text-danger">{error("rrule")}</p>
      {/if}
      {#if preset !== "none" && preset !== "custom" && endsKind === "count"}
        <NumberField label={m.event_count()} min={1} step={1} bind:value={count} />
      {:else if preset !== "none" && preset !== "custom" && endsKind === "until"}
        <DatePicker label={m.event_until()} bind:value={until} />
      {/if}
      <div class="flex flex-col gap-2">
        <span class="text-sm font-medium text-fg">{m.event_reminders()}</span>
        {#if reminders.length > 0}
          <ul class="flex flex-wrap gap-2">
            {#each reminders as minutes (minutes)}
              <li
                class="flex items-center gap-1 rounded-[var(--radius)] bg-surface px-2 py-1 text-sm"
              >
                {reminderLabel(minutes)}
                <button
                  type="button"
                  aria-label={m.reminder_remove({ label: reminderLabel(minutes) })}
                  class="rounded text-muted hover:text-fg focus-visible:outline-2 focus-visible:outline-focus"
                  onclick={() => (reminders = reminders.filter((r) => r !== minutes))}
                >
                  <X class="size-3.5" aria-hidden="true" />
                </button>
              </li>
            {/each}
          </ul>
        {/if}
        <div class="max-w-xs">
          <Select
            label={m.reminder_add()}
            options={reminderOptions}
            bind:value={
              () => null, (v) => v && (reminders = [...reminders, Number(v)].sort((a, b) => a - b))
            }
          />
        </div>
      </div>
      <TextField label={m.event_location()} bind:value={draft.location} />
      <div class="grid gap-4 sm:grid-cols-2">
        <Select
          label={m.event_client()}
          options={clientOptions}
          bind:value={
            () => draft.clientId ?? NONE,
            (v) => {
              draft.clientId = v || null;
              if (project && project.clientId !== draft.clientId) draft.projectId = null;
            }
          }
        />
        <Select
          label={m.event_project()}
          options={projectOptions}
          bind:value={
            () => draft.projectId ?? NONE,
            (v) => {
              draft.projectId = v || null;
              const linked = projects.find((p) => p.id === draft.projectId);
              if (linked) draft.clientId = linked.clientId;
            }
          }
        />
      </div>
      <TextArea label={m.event_description()} rows={3} bind:value={draft.descriptionMd} />
    </form>
    {#snippet footer()}
      {#if seed.event}
        <Button variant="ghost" onclick={remove}>{m.event_delete()}</Button>
      {/if}
      <span class="flex-1"></span>
      <Button onclick={onclose}>{m.ui_cancel()}</Button>
      <Button type="submit" form="event-form" variant="primary" loading={saving}
        >{m.ui_save()}</Button
      >
    {/snippet}
  </Dialog>
{/if}
