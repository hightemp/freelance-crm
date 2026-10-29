<script lang="ts">
  import {
    commands,
    type Project_Serialize as Project,
    type Task_Serialize as Task,
    type TimeEntryInput,
  } from "$lib/api/bindings";
  import { call } from "$lib/api/client";
  import { m } from "$lib/paraglide/messages";
  import Button from "$lib/ui/Button.svelte";
  import Checkbox from "$lib/ui/Checkbox.svelte";
  import DateTimePicker from "$lib/ui/DateTimePicker.svelte";
  import Dialog from "$lib/ui/Dialog.svelte";
  import Select from "$lib/ui/Select.svelte";
  import TextField from "$lib/ui/TextField.svelte";
  import { fromLocal } from "./format";

  let {
    date,
    projects,
    onsave,
    onclose,
  }: {
    /** Day of the new entry. */
    date: string;
    projects: Project[];
    onsave: (input: TimeEntryInput) => Promise<boolean>;
    onclose: () => void;
  } = $props();

  const NONE = "";
  // The day only seeds the form.
  // svelte-ignore state_referenced_locally
  const seed = { date };
  let draft = $state({
    description: "",
    start: `${seed.date}T09:00`,
    end: `${seed.date}T10:00`,
    billable: true,
    projectId: null as string | null,
    taskId: null as string | null,
  });
  let tasks = $state<Task[]>([]);
  let saving = $state(false);

  $effect(() => {
    const projectId = draft.projectId;
    if (!projectId) {
      tasks = [];
      return;
    }
    void call(
      commands.listTasks({
        statusIds: [],
        tagIds: [],
        scope: { type: "project", id: projectId },
        hideDone: false,
        sort: "board",
      }),
    ).then((list) => (tasks = list));
  });

  async function save(event: SubmitEvent) {
    event.preventDefault();
    saving = true;
    const saved = await onsave({
      target: { taskId: draft.taskId, projectId: draft.projectId, clientId: null },
      description: draft.description,
      startedAt: fromLocal(draft.start),
      endedAt: fromLocal(draft.end),
      billable: draft.billable,
    });
    saving = false;
    if (saved) onclose();
  }
</script>

<Dialog open title={m.time_add()} {onclose}>
  <form id="time-entry-form" class="flex w-[32rem] max-w-full flex-col gap-4" onsubmit={save}>
    <div class="grid gap-3 sm:grid-cols-2">
      <Select
        label={m.time_project()}
        options={[
          { value: NONE, label: m.time_no_project() },
          ...projects.map((p) => ({ value: p.id, label: `${p.name} · ${p.clientName}` })),
        ]}
        bind:value={
          () => draft.projectId ?? NONE,
          (v) => {
            draft.projectId = v || null;
            draft.taskId = null;
          }
        }
      />
      {#if draft.projectId}
        <Select
          label={m.time_task()}
          options={[
            { value: NONE, label: m.time_no_task() },
            ...tasks.map((t) => ({ value: t.id, label: t.title })),
          ]}
          bind:value={() => draft.taskId ?? NONE, (v) => (draft.taskId = v || null)}
        />
      {/if}
    </div>
    <TextField label={m.time_description()} bind:value={draft.description} />
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
    <Checkbox label={m.time_billable()} bind:checked={draft.billable} />
  </form>
  {#snippet footer()}
    <Button onclick={onclose}>{m.ui_cancel()}</Button>
    <Button type="submit" form="time-entry-form" variant="primary" loading={saving}
      >{m.ui_save()}</Button
    >
  {/snippet}
</Dialog>
