<script lang="ts">
  import { onMount } from "svelte";
  import { resolve } from "$app/paths";
  import Ellipsis from "@lucide/svelte/icons/ellipsis";
  import GripVertical from "@lucide/svelte/icons/grip-vertical";
  import Plus from "@lucide/svelte/icons/plus";
  import {
    dndzone,
    dragHandle,
    dragHandleZone,
    setAriaStrings,
    TRIGGERS,
    type DndEvent,
  } from "svelte-dnd-action";
  import {
    commands,
    type StatusInput,
    type TaskScope,
    type TaskStatus_Serialize as TaskStatus,
    type Task_Serialize as Task,
  } from "$lib/api/bindings";
  import { call } from "$lib/api/client";
  import { errorMessage } from "$lib/i18n/errors";
  import { formatIsoDate } from "$lib/i18n/format";
  import { i18n } from "$lib/i18n/locale.svelte";
  import { m } from "$lib/paraglide/messages";
  import Button from "$lib/ui/Button.svelte";
  import DropdownMenu from "$lib/ui/DropdownMenu.svelte";
  import Spinner from "$lib/ui/Spinner.svelte";
  import { toast } from "$lib/ui/toast.svelte";
  import ColumnDialog from "./ColumnDialog.svelte";
  import DeleteColumnDialog from "./DeleteColumnDialog.svelte";
  import { priorityLabel } from "./labels";
  import { statusName } from "./statuses";

  let { scope }: { scope?: TaskScope } = $props();

  interface Lane {
    /** Column id: svelte-dnd-action identifies items by `id`. */
    id: string;
    column: TaskStatus;
    tasks: Task[];
  }

  const FLIP_MS = 150;

  let lanes = $state<Lane[] | null>(null);
  /** Column dialog: `null` closed, `undefined` column for a new one. */
  let editing = $state<{ column?: TaskStatus } | null>(null);
  let deleting = $state<Lane | null>(null);
  let busy = $state(false);

  const showError = (err: unknown) => toast.show({ kind: "error", message: errorMessage(err) });

  async function load() {
    try {
      const [columns, tasks] = await Promise.all([
        call(commands.listBoardColumns()),
        call(
          commands.listTasks({
            query: null,
            statusIds: [],
            priority: null,
            scope: scope ?? null,
            tagIds: [],
            due: null,
            hideDone: false,
            sort: "board",
          }),
        ),
      ]);
      // Counts are recomputed from the cards, which move optimistically.
      lanes = columns.map((c) => ({
        id: c.id,
        column: {
          id: c.id,
          name: c.name,
          nameKey: c.nameKey,
          category: c.category,
          color: c.color,
          position: c.position,
          wipLimit: c.wipLimit,
        },
        tasks: tasks.filter((t) => t.statusId === c.id),
      }));
    } catch (err) {
      showError(err);
    }
  }

  $effect(() => {
    void scope;
    void load();
  });

  onMount(() => {
    // Screen reader announcements of keyboard dragging, in the app language.
    setAriaStrings({
      dragStarted: ({ itemLabel }) => m.board_aria_drag_started({ item: itemLabel }),
      movedToPosition: ({ itemLabel, zoneLabel, position, count }) =>
        m.board_aria_moved({ item: itemLabel, zone: zoneLabel, position, count }),
      dropped: ({ itemLabel, zoneLabel, position, count }) =>
        m.board_aria_dropped({ item: itemLabel, zone: zoneLabel, position, count }),
      zoneActiveInstruction: () => m.board_aria_instruction(),
    });
    return () => setAriaStrings(null);
  });

  function considerCards(lane: Lane, e: CustomEvent<DndEvent<Task>>) {
    lane.tasks = e.detail.items;
  }

  /** Optimistic: the card stays where it was dropped; on failure the board reloads. */
  async function finalizeCards(lane: Lane, e: CustomEvent<DndEvent<Task>>) {
    const { items, info } = e.detail;
    // New objects: the event's items must not be mutated.
    lane.tasks = items.map((t) => (t.id === info.id ? { ...t, statusId: lane.column.id } : t));
    if (info.trigger !== TRIGGERS.DROPPED_INTO_ZONE) return;
    const index = items.findIndex((t) => t.id === info.id);
    if (index < 0) return;
    try {
      await call(
        commands.moveTask(
          info.id,
          lane.column.id,
          items[index - 1]?.id ?? null,
          items[index + 1]?.id ?? null,
        ),
      );
    } catch (err) {
      showError(err);
      await load();
    }
  }

  async function finalizeColumns(e: CustomEvent<DndEvent<Lane>>) {
    // Keep our lane objects; only the order comes from the event.
    const byId = new Map((lanes ?? []).map((l) => [l.id, l]));
    lanes = e.detail.items.map((item) => byId.get(item.id) ?? item);
    if (e.detail.info.trigger !== TRIGGERS.DROPPED_INTO_ZONE) return;
    try {
      await call(commands.reorderStatuses(lanes.map((l) => l.id)));
    } catch (err) {
      showError(err);
      await load();
    }
  }

  async function act(action: () => Promise<unknown>) {
    busy = true;
    try {
      await action();
      return true;
    } catch (err) {
      showError(err);
      return false;
    } finally {
      busy = false;
    }
  }

  async function saveColumn(input: StatusInput) {
    const column = editing?.column;
    const ok = await act(() =>
      column ? call(commands.updateStatus(column.id, input)) : call(commands.createStatus(input)),
    );
    if (ok) {
      editing = null;
      await load();
    }
  }

  async function deleteColumn(moveTo: string | null) {
    const lane = deleting;
    if (!lane) return;
    const ok = await act(() => call(commands.deleteStatus(lane.id, moveTo)));
    if (ok) {
      deleting = null;
      await load();
    }
  }

  function onColumnAction(lane: Lane, action: string) {
    if (action === "edit") editing = { column: lane.column };
    else if (action === "delete") deleting = lane;
  }

  function where(task: Task): string {
    if (!task.clientName) return "";
    return task.projectName ? `${task.clientName} · ${task.projectName}` : task.clientName;
  }
</script>

{#if lanes === null}
  <div class="flex justify-center p-8"><Spinner /></div>
{:else}
  <div class="flex h-full min-h-0 items-start gap-3 overflow-x-auto pb-2">
    <div
      class="flex h-full items-start gap-3"
      aria-label={m.board_label()}
      use:dragHandleZone={{ items: lanes, type: "column", flipDurationMs: FLIP_MS }}
      onconsider={(e) => (lanes = e.detail.items)}
      onfinalize={finalizeColumns}
    >
      {#each lanes as lane (lane.id)}
        {@const name = statusName(lane.column)}
        {@const limit = lane.column.wipLimit}
        {@const over = limit !== null && lane.tasks.length > limit}
        <div class="flex h-full w-72 shrink-0 flex-col" aria-label={name}>
          <section
            aria-labelledby="column-{lane.id}"
            class="flex max-h-full min-h-0 flex-col gap-2 rounded-[var(--radius)] bg-surface p-2"
          >
            <header class="flex items-center gap-1.5 px-1">
              <span
                use:dragHandle
                aria-label={m.column_drag({ name })}
                class="cursor-grab text-muted hover:text-fg"
              >
                <GripVertical class="size-4" aria-hidden="true" />
              </span>
              <span
                class="size-2.5 shrink-0 rounded-full"
                style:background-color={lane.column.color}
                aria-hidden="true"
              ></span>
              <h2
                id="column-{lane.id}"
                class="min-w-0 flex-1 truncate text-sm font-semibold text-fg"
              >
                {name}
              </h2>
              <span
                class="text-xs tabular-nums {over ? 'font-semibold text-danger' : 'text-muted'}"
              >
                {limit === null
                  ? lane.tasks.length
                  : m.column_count_limit({ count: lane.tasks.length, limit })}
              </span>
              <DropdownMenu
                items={[
                  { id: "edit", label: m.column_edit() },
                  { separator: true },
                  { id: "delete", label: m.column_delete(), danger: true },
                ]}
                onSelect={(action) => onColumnAction(lane, action)}
                placement="bottom-end"
              >
                {#snippet trigger(props)}
                  <Button
                    {...props}
                    variant="ghost"
                    size="sm"
                    aria-label={m.column_actions({ name })}
                  >
                    <Ellipsis class="size-4" aria-hidden="true" />
                  </Button>
                {/snippet}
              </DropdownMenu>
            </header>
            {#if over}
              <p class="px-1 text-xs text-danger">{m.column_over_limit()}</p>
            {/if}
            <div
              class="flex min-h-12 flex-col gap-2 overflow-y-auto"
              aria-label={m.column_tasks({ name })}
              use:dndzone={{ items: lane.tasks, type: "task", flipDurationMs: FLIP_MS }}
              onconsider={(e) => considerCards(lane, e)}
              onfinalize={(e) => finalizeCards(lane, e)}
            >
              {#each lane.tasks as task (task.id)}
                <!-- A div: svelte-dnd-action gives cards the listitem role. -->
                <div
                  class="flex flex-col gap-1 rounded-[var(--radius)] border border-border bg-bg p-2 text-sm"
                  aria-label={task.title}
                >
                  <h3 class="text-fg">
                    <a
                      href={resolve("/tasks/[id]", { id: task.id })}
                      class="hover:underline focus-visible:outline-2 focus-visible:outline-focus"
                    >
                      {task.title}
                    </a>
                  </h3>
                  <div class="flex flex-wrap gap-x-2 text-xs text-muted">
                    {#if task.priority !== "normal"}
                      <span class={task.priority === "urgent" ? "text-danger" : ""}>
                        {priorityLabel(task.priority)}
                      </span>
                    {/if}
                    {#if where(task)}<span>{where(task)}</span>{/if}
                    {#if task.dueDate}<span>{formatIsoDate(task.dueDate, i18n.locale)}</span>{/if}
                    {#if task.checklistTotal > 0}
                      <span class="tabular-nums">
                        {m.task_checklist_progress({
                          done: task.checklistDone,
                          total: task.checklistTotal,
                        })}
                      </span>
                    {/if}
                  </div>
                </div>
              {/each}
            </div>
          </section>
        </div>
      {/each}
    </div>
    <Button variant="ghost" onclick={() => (editing = {})}>
      <Plus class="size-4" aria-hidden="true" />
      {m.board_add_column()}
    </Button>
  </div>
{/if}

{#if editing}
  <ColumnDialog
    column={editing.column}
    saving={busy}
    onsave={saveColumn}
    oncancel={() => (editing = null)}
  />
{/if}

{#if deleting && lanes}
  {@const lane = deleting}
  <DeleteColumnDialog
    column={lane.column}
    taskCount={lane.tasks.length}
    others={lanes.filter((l) => l.id !== lane.id).map((l) => l.column)}
    {busy}
    ondelete={deleteColumn}
    oncancel={() => (deleting = null)}
  />
{/if}
