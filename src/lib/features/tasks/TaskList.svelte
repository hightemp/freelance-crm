<script lang="ts">
  import {
    commands,
    type BoardColumn,
    type DuePreset,
    type Priority,
    type TaskFilter_Deserialize as TaskFilter,
    type TaskScope,
    type TaskSort,
    type Task,
  } from "$lib/api/bindings";
  import { call } from "$lib/api/client";
  import { errorMessage } from "$lib/i18n/errors";
  import { formatIsoDate } from "$lib/i18n/format";
  import { i18n } from "$lib/i18n/locale.svelte";
  import { m } from "$lib/paraglide/messages";
  import Checkbox from "$lib/ui/Checkbox.svelte";
  import Select from "$lib/ui/Select.svelte";
  import Spinner from "$lib/ui/Spinner.svelte";
  import Switch from "$lib/ui/Switch.svelte";
  import TextField from "$lib/ui/TextField.svelte";
  import VirtualList from "$lib/ui/VirtualList.svelte";
  import { toast } from "$lib/ui/toast.svelte";
  import { DUE_PRESETS, PRIORITIES, SORTS, dueLabel, priorityLabel, sortLabel } from "./labels";
  import { statusName } from "./statuses";

  let {
    scope,
    searchDelay = 300,
  }: {
    /** Only tasks of a project, a client or the inbox. */
    scope?: TaskScope;
    searchDelay?: number;
  } = $props();

  const ALL = "all";

  let search = $state("");
  let query = $state("");
  let column = $state<string | null>(ALL);
  let priority = $state<string | null>(ALL);
  let due = $state<string | null>(ALL);
  let sort = $state<string | null>("board");
  let hideDone = $state(false);
  let tasks = $state<Task[] | null>(null);
  let columns = $state<BoardColumn[]>([]);

  let columnsById = $derived(new Map(columns.map((c) => [c.id, c])));
  let firstTodo = $derived(columns.find((c) => c.category === "todo"));
  let firstDone = $derived(columns.find((c) => c.category === "done"));
  let filtered = $derived(
    query !== "" || column !== ALL || priority !== ALL || due !== ALL || hideDone,
  );

  let columnOptions = $derived([
    { value: ALL, label: m.tasks_column_all() },
    ...columns.map((c) => ({ value: c.id, label: statusName(c) })),
  ]);
  let priorityOptions = $derived([
    { value: ALL, label: m.tasks_priority_all() },
    ...PRIORITIES.map((p) => ({ value: p, label: priorityLabel(p) })),
  ]);
  let dueOptions = $derived([
    { value: ALL, label: m.tasks_due_all() },
    ...DUE_PRESETS.map((d) => ({ value: d, label: dueLabel(d) })),
  ]);
  let sortOptions = $derived(SORTS.map((s) => ({ value: s, label: sortLabel(s) })));

  const showError = (err: unknown) => toast.show({ kind: "error", message: errorMessage(err) });

  $effect(() => {
    const next = search.trim();
    const timer = setTimeout(() => (query = next), searchDelay);
    return () => clearTimeout(timer);
  });

  let latest = 0;
  function load() {
    const filter: TaskFilter = {
      query: query || null,
      statusIds: column === ALL || !column ? [] : [column],
      priority: priority === ALL ? null : (priority as Priority),
      scope: scope ?? null,
      tagIds: [],
      due: due === ALL ? null : (due as DuePreset),
      hideDone,
      sort: (sort ?? "board") as TaskSort,
    };
    const request = ++latest;
    call(commands.listTasks(filter)).then((result) => {
      if (request === latest) tasks = result;
    }, showError);
  }

  $effect(load);

  $effect(() => {
    call(commands.listBoardColumns()).then((result) => (columns = result), showError);
  });

  async function toggle(task: Task, done: boolean) {
    const target = done ? firstDone : firstTodo;
    if (!target) return;
    try {
      await call(commands.setTaskStatus(task.id, target.id));
      load();
    } catch (err) {
      showError(err);
    }
  }

  function where(task: Task): string {
    if (!task.clientName) return m.task_inbox();
    return task.projectName ? `${task.clientName} · ${task.projectName}` : task.clientName;
  }

  const priorityClass: Record<Priority, string> = {
    urgent: "text-danger",
    high: "text-accent",
    normal: "text-muted",
    low: "text-muted",
  };
</script>

<div class="flex h-full min-h-0 flex-col gap-4">
  <div class="flex flex-wrap items-end gap-3">
    <div class="w-56">
      <TextField label={m.tasks_search()} bind:value={search} />
    </div>
    <div class="w-40">
      <Select label={m.tasks_column()} options={columnOptions} bind:value={column} />
    </div>
    <div class="w-40">
      <Select label={m.tasks_priority()} options={priorityOptions} bind:value={priority} />
    </div>
    <div class="w-44">
      <Select label={m.tasks_due()} options={dueOptions} bind:value={due} />
    </div>
    <div class="w-40">
      <Select label={m.tasks_sort()} options={sortOptions} bind:value={sort} />
    </div>
    <div class="pb-1.5">
      <Switch label={m.tasks_hide_done()} bind:checked={hideDone} />
    </div>
  </div>

  {#if tasks === null}
    <div class="flex justify-center p-8"><Spinner /></div>
  {:else if tasks.length === 0}
    <div
      class="flex flex-col items-center gap-2 rounded-[var(--radius)] border border-dashed border-border p-10 text-center"
    >
      {#if filtered}
        <p class="text-sm font-medium text-fg">{m.tasks_no_matches()}</p>
      {:else}
        <p class="text-sm font-medium text-fg">{m.tasks_empty_title()}</p>
        <p class="max-w-sm text-sm text-muted">{m.tasks_empty_hint()}</p>
      {/if}
    </div>
  {:else}
    <div class="min-h-0 flex-1 rounded-[var(--radius)] border border-border">
      <VirtualList label={m.tasks_list()} items={tasks} key={(t) => t.id} rowHeight={44}>
        {#snippet row(task)}
          {@const status = columnsById.get(task.statusId)}
          <div class="flex h-full items-center gap-3 border-b border-border px-3 text-sm">
            <div class="min-w-0 flex-1 truncate">
              <Checkbox
                label={task.title}
                bind:checked={() => status?.category === "done", (done) => void toggle(task, done)}
              />
            </div>
            <span class="w-20 shrink-0 text-xs font-medium {priorityClass[task.priority]}">
              {priorityLabel(task.priority)}
            </span>
            <span class="w-28 shrink-0 truncate text-xs text-muted">
              {status ? statusName(status) : ""}
            </span>
            <span class="w-48 shrink-0 truncate text-xs text-muted">{where(task)}</span>
            <span class="w-24 shrink-0 text-xs whitespace-nowrap text-muted">
              {task.dueDate ? formatIsoDate(task.dueDate, i18n.locale) : ""}
            </span>
            <span class="w-10 shrink-0 text-right text-xs text-muted tabular-nums">
              {task.checklistTotal > 0
                ? m.task_checklist_progress({
                    done: task.checklistDone,
                    total: task.checklistTotal,
                  })
                : ""}
            </span>
          </div>
        {/snippet}
      </VirtualList>
    </div>
  {/if}
</div>
