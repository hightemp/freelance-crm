<script lang="ts">
  import Ellipsis from "@lucide/svelte/icons/ellipsis";
  import Pencil from "@lucide/svelte/icons/pencil";
  import { goto } from "$app/navigation";
  import { resolve } from "$app/paths";
  import {
    commands,
    type BoardColumn,
    type EntityRef,
    type Task_Serialize as Task,
  } from "$lib/api/bindings";
  import { call } from "$lib/api/client";
  import { errorMessage } from "$lib/i18n/errors";
  import { formatIsoDate } from "$lib/i18n/format";
  import { i18n } from "$lib/i18n/locale.svelte";
  import Markdown from "$lib/markdown/Markdown.svelte";
  import { m } from "$lib/paraglide/messages";
  import Button from "$lib/ui/Button.svelte";
  import { confirm } from "$lib/ui/dialogs.svelte";
  import DropdownMenu from "$lib/ui/DropdownMenu.svelte";
  import Select from "$lib/ui/Select.svelte";
  import Spinner from "$lib/ui/Spinner.svelte";
  import { toast } from "$lib/ui/toast.svelte";
  import NotesPanel from "../notes/NotesPanel.svelte";
  import TagEditor from "../tags/TagEditor.svelte";
  import ChecklistPanel from "./ChecklistPanel.svelte";
  import { priorityLabel } from "./labels";
  import { statusName } from "./statuses";

  let { id }: { id: string } = $props();

  let task = $state<Task | null>(null);
  let columns = $state<BoardColumn[]>([]);
  let loadError = $state<string | null>(null);

  let owner = $derived<EntityRef>({ type: "task", id });
  let columnOptions = $derived(columns.map((c) => ({ value: c.id, label: statusName(c) })));

  const showError = (err: unknown) => toast.show({ kind: "error", message: errorMessage(err) });

  async function reload() {
    try {
      task = await call(commands.getTask(id));
    } catch (err) {
      loadError = errorMessage(err);
    }
  }

  $effect(() => {
    void id;
    loadError = null;
    void reload();
    call(commands.listBoardColumns()).then((c) => (columns = c), showError);
  });

  async function setStatus(statusId: string | null) {
    if (!statusId || !task || statusId === task.statusId) return;
    try {
      task = await call(commands.setTaskStatus(id, statusId));
    } catch (err) {
      showError(err);
    }
  }

  async function onAction(action: string) {
    if (action !== "delete" || !task) return;
    const ok = await confirm({
      title: m.task_delete_title({ name: task.title }),
      confirmLabel: m.ui_delete(),
      danger: true,
    });
    if (!ok) return;
    try {
      await call(commands.deleteTask(id));
      toast.show({ message: m.task_deleted_toast() });
      await goto(resolve("/tasks"));
    } catch (err) {
      showError(err);
    }
  }

  function deadline(t: Task): string {
    if (!t.dueDate) return m.client_not_set();
    const date = formatIsoDate(t.dueDate, i18n.locale);
    return t.dueTime ? `${date}, ${t.dueTime}` : date;
  }

  function estimate(minutes: number | null): string {
    if (minutes === null) return m.client_not_set();
    const hours = new Intl.NumberFormat(i18n.locale, { maximumFractionDigits: 2 }).format(
      minutes / 60,
    );
    return m.task_estimate_value({ hours });
  }
</script>

{#if loadError}
  <p class="text-sm text-danger" role="alert">{loadError}</p>
{:else if !task}
  <div class="flex justify-center p-8"><Spinner /></div>
{:else}
  <div class="flex max-w-3xl flex-col gap-6">
    <header class="flex items-start gap-3">
      <div class="min-w-0 flex-1">
        <h1 class="text-xl font-semibold text-fg">{task.title}</h1>
        <p class="flex flex-wrap gap-2 text-sm text-muted">
          {#if task.clientId}
            <a
              href={resolve("/clients/[id]", { id: task.clientId })}
              class="hover:underline focus-visible:outline-2 focus-visible:outline-focus"
            >
              {task.clientName}
            </a>
          {:else}
            <span>{m.task_inbox()}</span>
          {/if}
          {#if task.projectId}
            <span aria-hidden="true">·</span>
            <a
              href={resolve("/projects/[id]", { id: task.projectId })}
              class="hover:underline focus-visible:outline-2 focus-visible:outline-focus"
            >
              {task.projectName}
            </a>
          {/if}
          {#if task.completedAt !== null}
            <span aria-hidden="true">·</span>
            <span class="text-accent">{m.task_completed()}</span>
          {/if}
        </p>
      </div>
      <Button onclick={() => goto(resolve("/tasks/[id]/edit", { id }))}>
        <Pencil class="size-4" aria-hidden="true" />
        {m.ui_edit()}
      </Button>
      <DropdownMenu
        items={[{ id: "delete", label: m.task_delete(), danger: true }]}
        onSelect={onAction}
        placement="bottom-end"
      >
        {#snippet trigger(props)}
          <Button {...props} aria-label={m.task_actions()}>
            <Ellipsis class="size-4" aria-hidden="true" />
          </Button>
        {/snippet}
      </DropdownMenu>
    </header>

    <div class="grid gap-4 sm:grid-cols-[14rem_1fr]">
      <Select
        label={m.task_field_status()}
        options={columnOptions}
        bind:value={() => task?.statusId ?? null, setStatus}
      />
      <TagEditor {owner} label={m.client_tags()} bind:tagIds={task.tagIds} />
    </div>

    <dl class="grid grid-cols-[12rem_1fr] gap-x-4 gap-y-3 text-sm">
      <dt class="text-muted">{m.task_field_priority()}</dt>
      <dd class="text-fg">{priorityLabel(task.priority)}</dd>
      <dt class="text-muted">{m.task_field_due_date()}</dt>
      <dd class="text-fg">{deadline(task)}</dd>
      <dt class="text-muted">{m.task_field_estimate()}</dt>
      <dd class="text-fg">{estimate(task.estimateMinutes)}</dd>
    </dl>

    <section class="flex flex-col gap-2">
      <h2 class="text-base font-semibold text-fg">{m.task_field_description()}</h2>
      {#if task.descriptionMd.trim()}
        <Markdown source={task.descriptionMd} />
      {:else}
        <p class="text-sm text-muted">{m.project_no_description()}</p>
      {/if}
    </section>

    <ChecklistPanel taskId={id} onchange={reload} />

    <section class="flex flex-col gap-2">
      <h2 class="text-base font-semibold text-fg">{m.client_tab_notes()}</h2>
      <NotesPanel {owner} />
    </section>
  </div>
{/if}
