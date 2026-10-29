<script lang="ts">
  import { commands, type Task_Serialize as Task } from "$lib/api/bindings";
  import { call } from "$lib/api/client";
  import { errorMessage } from "$lib/i18n/errors";
  import { m } from "$lib/paraglide/messages";
  import Spinner from "$lib/ui/Spinner.svelte";
  import TaskForm from "./TaskForm.svelte";

  let {
    id,
    onsaved,
    oncancel,
  }: { id: string; onsaved: (task: Task) => void; oncancel: () => void } = $props();

  let task = $state<Task | null>(null);
  let loadError = $state<string | null>(null);

  $effect(() => {
    call(commands.getTask(id)).then(
      (t) => (task = t),
      (err) => (loadError = errorMessage(err)),
    );
  });
</script>

<div class="flex flex-col gap-6">
  <h1 class="text-xl font-semibold text-fg">{m.task_edit()}</h1>
  {#if loadError}
    <p class="text-sm text-danger" role="alert">{loadError}</p>
  {:else if task}
    <TaskForm {task} {onsaved} {oncancel} />
  {:else}
    <div class="flex justify-center p-8"><Spinner /></div>
  {/if}
</div>
