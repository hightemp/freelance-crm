<script lang="ts">
  import {
    commands,
    type BoardColumn,
    type Client_Serialize as Client,
    type Priority,
    type Project_Serialize as Project,
    type TaskInput_Deserialize as TaskInput,
    type Task_Serialize as Task,
  } from "$lib/api/bindings";
  import { AppError, call } from "$lib/api/client";
  import { codeMessage, errorMessage } from "$lib/i18n/errors";
  import MarkdownEditor from "$lib/markdown/MarkdownEditor.svelte";
  import { m } from "$lib/paraglide/messages";
  import Button from "$lib/ui/Button.svelte";
  import DatePicker from "$lib/ui/DatePicker.svelte";
  import NumberField from "$lib/ui/NumberField.svelte";
  import Select from "$lib/ui/Select.svelte";
  import TextField from "$lib/ui/TextField.svelte";
  import TimePicker from "$lib/ui/TimePicker.svelte";
  import { toast } from "$lib/ui/toast.svelte";
  import { PRIORITIES, priorityLabel } from "./labels";
  import { statusName } from "./statuses";

  let {
    task,
    projectId,
    clientId,
    onsaved,
    oncancel,
  }: {
    /** Task to edit; a new one is created when omitted. */
    task?: Task;
    /** Links of a new task, e.g. when created from a project card. */
    projectId?: string;
    clientId?: string;
    onsaved: (task: Task) => void;
    oncancel?: () => void;
  } = $props();

  const NONE = "";

  // The props only seed the form.
  // svelte-ignore state_referenced_locally
  const seed = { task, projectId, clientId };
  let draft = $state<TaskInput>({
    title: seed.task?.title ?? "",
    descriptionMd: seed.task?.descriptionMd ?? "",
    statusId: seed.task?.statusId ?? null,
    priority: seed.task?.priority ?? "normal",
    projectId: seed.task?.projectId ?? seed.projectId ?? null,
    clientId: seed.task?.clientId ?? seed.clientId ?? null,
    dueDate: seed.task?.dueDate ?? null,
    dueTime: seed.task?.dueTime ?? null,
    estimateMinutes: seed.task?.estimateMinutes ?? null,
  });
  let estimateHours = $state<number | null>(
    seed.task?.estimateMinutes != null ? seed.task.estimateMinutes / 60 : null,
  );
  let savedId = $state(seed.task?.id);
  let columns = $state<BoardColumn[]>([]);
  let clients = $state<Client[]>([]);
  let projects = $state<Project[]>([]);
  let errors = $state<Record<string, string>>({});
  let saving = $state(false);

  let project = $derived(projects.find((p) => p.id === draft.projectId));
  // A project decides the client; a client narrows the projects.
  let clientOptions = $derived(
    project
      ? [{ value: project.clientId, label: project.clientName }]
      : [
          { value: NONE, label: m.task_no_client() },
          ...clients.map((c) => ({ value: c.id, label: c.displayName })),
        ],
  );
  let projectOptions = $derived([
    { value: NONE, label: m.task_no_project() },
    ...projects
      .filter((p) => !draft.clientId || p.clientId === draft.clientId)
      .map((p) => ({ value: p.id, label: p.name })),
  ]);
  let columnOptions = $derived(columns.map((c) => ({ value: c.id, label: statusName(c) })));
  let priorityOptions = $derived(PRIORITIES.map((p) => ({ value: p, label: priorityLabel(p) })));

  $effect(() => {
    Promise.all([
      call(commands.listBoardColumns()),
      call(commands.listClients({ query: null, status: null, archived: false, tagIds: [] })),
      call(commands.listProjects({ query: null, status: null, clientId: null, archived: false })),
    ]).then(
      ([cols, cls, prs]) => {
        columns = cols;
        clients = cls;
        projects = prs;
        draft.statusId ??= cols.find((c) => c.category === "todo")?.id ?? null;
        const linked = prs.find((p) => p.id === draft.projectId);
        if (linked) draft.clientId = linked.clientId;
      },
      (err) => toast.show({ kind: "error", message: errorMessage(err) }),
    );
  });

  function setProject(id: string | null) {
    draft.projectId = id || null;
    const linked = projects.find((p) => p.id === draft.projectId);
    if (linked) draft.clientId = linked.clientId;
  }

  function setClient(id: string | null) {
    draft.clientId = id || null;
    if (project && project.clientId !== draft.clientId) draft.projectId = null;
  }

  const error = (field: string) => (errors[field] ? codeMessage(errors[field]) : undefined);

  function validate(): Record<string, string> {
    const result: Record<string, string> = {};
    if (draft.title.trim() === "") result.title = "validation.required";
    if (draft.dueTime && !draft.dueDate) result.dueTime = "validation.needs_date";
    return result;
  }

  async function save(event: SubmitEvent) {
    event.preventDefault();
    errors = validate();
    if (Object.keys(errors).length > 0) return;
    const input: TaskInput = {
      ...draft,
      title: draft.title.trim(),
      estimateMinutes:
        estimateHours !== null && estimateHours > 0 ? Math.round(estimateHours * 60) : null,
    };
    saving = true;
    try {
      const saved = savedId
        ? await call(commands.updateTask(savedId, input))
        : await call(commands.createTask(input));
      savedId = saved.id;
      onsaved(saved);
    } catch (err) {
      const field = err instanceof AppError ? err.params.field : undefined;
      if (err instanceof AppError && typeof field === "string") errors = { [field]: err.code };
      else toast.show({ kind: "error", message: errorMessage(err) });
    } finally {
      saving = false;
    }
  }
</script>

<form class="flex max-w-2xl flex-col gap-6" novalidate onsubmit={save}>
  <TextField
    label={m.task_field_title()}
    required
    bind:value={draft.title}
    error={error("title")}
  />
  <div class="grid gap-4 sm:grid-cols-2">
    <Select
      label={m.task_field_status()}
      options={columnOptions}
      bind:value={() => draft.statusId, (v) => (draft.statusId = v)}
    />
    <Select
      label={m.task_field_priority()}
      options={priorityOptions}
      bind:value={() => draft.priority, (v) => (draft.priority = v as Priority)}
    />
    <Select
      label={m.task_field_client()}
      options={clientOptions}
      bind:value={() => draft.clientId ?? NONE, setClient}
    />
    <Select
      label={m.task_field_project()}
      options={projectOptions}
      bind:value={() => draft.projectId ?? NONE, setProject}
    />
  </div>
  <div class="grid gap-4 sm:grid-cols-3">
    <DatePicker
      label={m.task_field_due_date()}
      bind:value={draft.dueDate}
      error={error("dueDate")}
    />
    <TimePicker
      label={m.task_field_due_time()}
      bind:value={draft.dueTime}
      error={error("dueTime")}
    />
    <NumberField label={m.task_field_estimate()} min={0} step={0.5} bind:value={estimateHours} />
  </div>
  <MarkdownEditor label={m.task_field_description()} bind:value={draft.descriptionMd} />
  <div class="flex gap-2">
    <Button type="submit" variant="primary" loading={saving}>{m.ui_save()}</Button>
    {#if oncancel}
      <Button onclick={oncancel}>{m.ui_cancel()}</Button>
    {/if}
  </div>
</form>
