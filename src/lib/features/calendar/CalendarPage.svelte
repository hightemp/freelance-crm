<script lang="ts">
  import ChevronLeft from "@lucide/svelte/icons/chevron-left";
  import ChevronRight from "@lucide/svelte/icons/chevron-right";
  import Plus from "@lucide/svelte/icons/plus";
  import { goto } from "$app/navigation";
  import { resolve } from "$app/paths";
  import {
    commands,
    type CalendarItem_Serialize as CalendarItem,
    type CalendarLayers,
  } from "$lib/api/bindings";
  import { call } from "$lib/api/client";
  import { errorMessage } from "$lib/i18n/errors";
  import { i18n } from "$lib/i18n/locale.svelte";
  import { m } from "$lib/paraglide/messages";
  import Button from "$lib/ui/Button.svelte";
  import Checkbox from "$lib/ui/Checkbox.svelte";
  import { addDays, longDate, monthTitle, todayIso } from "$lib/ui/date";
  import Popover from "$lib/ui/Popover.svelte";
  import { uiPrefs } from "$lib/ui/prefs.svelte";
  import { toast } from "$lib/ui/toast.svelte";
  import AgendaView from "./AgendaView.svelte";
  import EventDialog from "./EventDialog.svelte";
  import ExportDialog from "./ExportDialog.svelte";
  import { dateSpan } from "./format";
  import ImportDialog from "./ImportDialog.svelte";
  import { systemTimeZone } from "./labels";
  import MonthView from "./MonthView.svelte";
  import { viewRange, type CalendarView } from "./time";
  import TimeGrid from "./TimeGrid.svelte";

  let {
    today = todayIso(),
    timeZone = systemTimeZone(),
  }: {
    /** Where the calendar opens and "Today" returns. */
    today?: string;
    timeZone?: string;
  } = $props();

  // `today` only sets where the calendar opens.
  // svelte-ignore state_referenced_locally
  const start = { today };
  let view = $state<CalendarView>("month");
  let anchor = $state(start.today);
  let layers = $state<CalendarLayers>({
    events: true,
    tasks: true,
    projects: true,
    invoices: true,
    showCompleted: false,
  });
  let items = $state<CalendarItem[]>([]);
  let dialog = $state<
    | { kind: "event"; props: Record<string, unknown> }
    | { kind: "import" }
    | { kind: "export" }
    | null
  >(null);

  let range = $derived(viewRange(view, anchor, uiPrefs.weekStart));
  let title = $derived.by(() => {
    switch (view) {
      case "month":
        return monthTitle(anchor, i18n.locale);
      case "day":
        return longDate(anchor, i18n.locale);
      default:
        return dateSpan(range.days[0], range.days[range.days.length - 1], i18n.locale);
    }
  });

  async function load() {
    try {
      items = await call(commands.calendarItems(range.from, range.to, { ...layers }));
    } catch (err) {
      toast.show({ kind: "error", message: errorMessage(err) });
    }
  }

  $effect(() => {
    void [
      range.from,
      range.to,
      layers.events,
      layers.tasks,
      layers.projects,
      layers.invoices,
      layers.showCompleted,
    ];
    void load();
  });

  const views: { value: CalendarView; label: () => string }[] = [
    { value: "month", label: m.calendar_view_month },
    { value: "week", label: m.calendar_view_week },
    { value: "day", label: m.calendar_view_day },
    { value: "agenda", label: m.calendar_view_agenda },
  ];

  function open(item: CalendarItem) {
    if (item.kind === "event" && item.occurrence) {
      const o = item.occurrence;
      dialog = {
        kind: "event",
        props: {
          event: o.event,
          occurrence: { recurrenceId: o.recurrenceId, start: o.start, end: o.end },
        },
      };
    } else if (item.kind === "task_due") {
      void goto(resolve("/tasks/[id]", { id: item.id }));
    } else if (item.kind === "project_deadline") {
      void goto(resolve("/projects/[id]", { id: item.id }));
    }
  }

  /** A new event at a slot (`YYYY-MM-DDTHH:MM`) or on a day. */
  function create(at: string) {
    const startAt = at.includes("T") ? at : `${at}T09:00`;
    const [date, time] = startAt.split("T");
    const [h, min] = time.split(":").map(Number);
    const endMinutes = h * 60 + min + 60;
    const endDate = endMinutes >= 1440 ? addDays(date, 1) : date;
    const pad = (n: number) => String(n).padStart(2, "0");
    const end = `${endDate}T${pad(Math.floor((endMinutes % 1440) / 60))}:${pad(endMinutes % 60)}`;
    dialog = { kind: "event", props: { initial: { start: startAt, end, allDay: false } } };
  }

  function done() {
    dialog = null;
    void load();
  }

  const layerOptions: { key: keyof CalendarLayers; label: () => string }[] = [
    { key: "events", label: m.calendar_layer_events },
    { key: "tasks", label: m.calendar_layer_tasks },
    { key: "projects", label: m.calendar_layer_projects },
    { key: "invoices", label: m.calendar_layer_invoices },
    { key: "showCompleted", label: m.calendar_show_completed },
  ];
</script>

<div class="flex flex-col gap-4">
  <div class="flex flex-wrap items-center gap-2">
    <Button onclick={() => (anchor = today)}>{m.calendar_today()}</Button>
    <Button
      variant="ghost"
      aria-label={m.calendar_previous()}
      onclick={() => (anchor = range.previous)}
    >
      <ChevronLeft class="size-4" aria-hidden="true" />
    </Button>
    <Button variant="ghost" aria-label={m.calendar_next()} onclick={() => (anchor = range.next)}>
      <ChevronRight class="size-4" aria-hidden="true" />
    </Button>
    <h2 class="text-base font-semibold text-fg first-letter:uppercase">{title}</h2>
    <span class="flex-1"></span>
    <div role="group" aria-label={m.calendar_view()} class="flex gap-1">
      {#each views as v (v.value)}
        <Button
          size="sm"
          variant={view === v.value ? "secondary" : "ghost"}
          aria-pressed={view === v.value}
          onclick={() => (view = v.value)}
        >
          {v.label()}
        </Button>
      {/each}
    </div>
    <Popover label={m.calendar_layers()}>
      {#snippet trigger(props)}<Button size="sm" {...props}>{m.calendar_layers()}</Button>{/snippet}
      <div class="flex flex-col gap-2 p-1">
        {#each layerOptions as layer (layer.key)}
          <Checkbox label={layer.label()} bind:checked={layers[layer.key]} />
        {/each}
      </div>
    </Popover>
    <Button size="sm" onclick={() => (dialog = { kind: "import" })}>{m.calendar_import()}</Button>
    <Button size="sm" onclick={() => (dialog = { kind: "export" })}>{m.calendar_export()}</Button>
    <Button size="sm" variant="primary" onclick={() => create(`${anchor}T09:00`)}>
      <Plus class="size-4" aria-hidden="true" />
      {m.calendar_new_event()}
    </Button>
  </div>

  {#if view === "month"}
    <MonthView
      days={range.days}
      month={anchor.slice(0, 7)}
      {today}
      {items}
      onopen={open}
      onslot={create}
      onmore={(date) => {
        anchor = date;
        view = "day";
      }}
    />
  {:else if view === "agenda"}
    <AgendaView days={range.days} {items} onopen={open} />
  {:else}
    <TimeGrid days={range.days} {today} {items} onopen={open} onslot={create} />
  {/if}
</div>

{#if dialog?.kind === "event"}
  <EventDialog {...dialog.props} {timeZone} onsaved={done} onclose={() => (dialog = null)} />
{:else if dialog?.kind === "import"}
  <ImportDialog onclose={done} />
{:else if dialog?.kind === "export"}
  <ExportDialog onclose={() => (dialog = null)} />
{/if}
