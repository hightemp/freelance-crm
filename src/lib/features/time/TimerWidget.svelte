<script lang="ts">
  import Play from "@lucide/svelte/icons/play";
  import Square from "@lucide/svelte/icons/square";
  import {
    commands,
    type Project_Serialize as Project,
    type Task_Serialize as Task,
  } from "$lib/api/bindings";
  import { call } from "$lib/api/client";
  import { errorMessage } from "$lib/i18n/errors";
  import { m } from "$lib/paraglide/messages";
  import Button from "$lib/ui/Button.svelte";
  import Checkbox from "$lib/ui/Checkbox.svelte";
  import Popover from "$lib/ui/Popover.svelte";
  import Select from "$lib/ui/Select.svelte";
  import TextField from "$lib/ui/TextField.svelte";
  import { toast } from "$lib/ui/toast.svelte";
  import {
    formatElapsed,
    refreshTimer,
    startTimer,
    timerPanel,
    stopTimer,
    timer,
    watchTimer,
  } from "./timer.svelte";

  const NONE = "";
  const showError = (err: unknown) => toast.show({ kind: "error", message: errorMessage(err) });

  let now = $state(Date.now());
  let description = $state("");
  let projectId = $state<string | null>(null);
  let taskId = $state<string | null>(null);
  let billable = $state(true);
  let projects = $state<Project[]>([]);
  let tasks = $state<Task[]>([]);
  let busy = $state(false);

  $effect(() => {
    refreshTimer().catch(showError);
    return watchTimer(showError);
  });

  // Tick while running.
  $effect(() => {
    if (!timer.running) return;
    now = Date.now();
    const id = setInterval(() => (now = Date.now()), 1000);
    return () => clearInterval(id);
  });

  // Lists are loaded when the start panel opens.
  $effect(() => {
    if (!timerPanel.open) return;
    call(commands.listProjects({ query: null, status: null, clientId: null, archived: false }))
      .then((list) => (projects = list))
      .catch(showError);
  });

  $effect(() => {
    if (!projectId) {
      tasks = [];
      return;
    }
    call(
      commands.listTasks({
        statusIds: [],
        tagIds: [],
        scope: { type: "project", id: projectId },
        hideDone: true,
        sort: "board",
      }),
    )
      .then((list) => (tasks = list))
      .catch(showError);
  });

  let projectOptions = $derived([
    { value: NONE, label: m.timer_no_project() },
    ...projects.map((p) => ({ value: p.id, label: `${p.name} · ${p.clientName}` })),
  ]);
  let taskOptions = $derived([
    { value: NONE, label: m.timer_no_task() },
    ...tasks.map((t) => ({ value: t.id, label: t.title })),
  ]);

  let caption = $derived.by(() => {
    const r = timer.running;
    if (!r) return "";
    return [r.description, r.taskTitle ?? r.projectName ?? r.clientName]
      .filter(Boolean)
      .join(" · ");
  });

  async function start(event: SubmitEvent) {
    event.preventDefault();
    busy = true;
    try {
      await startTimer({ taskId, projectId, clientId: null }, description, billable);
      timerPanel.open = false;
      description = "";
    } catch (err) {
      showError(err);
    } finally {
      busy = false;
    }
  }

  async function stop() {
    busy = true;
    try {
      await stopTimer();
    } catch (err) {
      showError(err);
    } finally {
      busy = false;
    }
  }
</script>

{#if timer.running}
  <div class="flex items-center gap-2">
    <span
      role="timer"
      aria-label={m.timer_elapsed()}
      class="font-mono text-sm tabular-nums text-fg"
    >
      {formatElapsed(now - timer.running.startedAt)}
    </span>
    {#if caption}<span class="max-w-60 truncate text-sm text-muted">{caption}</span>{/if}
    <Button size="sm" variant="ghost" aria-label={m.timer_stop()} loading={busy} onclick={stop}>
      <Square class="size-4" aria-hidden="true" />
    </Button>
  </div>
{:else if timer.loaded}
  <Popover label={m.timer_start()} bind:open={timerPanel.open} placement="bottom-end">
    {#snippet trigger(props)}
      <Button size="sm" variant="ghost" {...props}>
        <Play class="size-4" aria-hidden="true" />
        {m.timer_start()}
      </Button>
    {/snippet}
    <form class="flex w-72 flex-col gap-3 p-1" onsubmit={start}>
      <TextField label={m.timer_description()} bind:value={description} />
      <Select
        label={m.timer_project()}
        options={projectOptions}
        bind:value={
          () => projectId ?? NONE,
          (v) => {
            projectId = v || null;
            taskId = null;
          }
        }
      />
      {#if projectId}
        <Select
          label={m.timer_task()}
          options={taskOptions}
          bind:value={() => taskId ?? NONE, (v) => (taskId = v || null)}
        />
      {/if}
      <Checkbox label={m.timer_billable()} bind:checked={billable} />
      <Button type="submit" variant="primary" loading={busy}>{m.timer_start_submit()}</Button>
    </form>
  </Popover>
{/if}
