<script lang="ts">
  import { goto } from "$app/navigation";
  import { resolve } from "$app/paths";
  import { page } from "$app/state";
  import TaskForm from "$lib/features/tasks/TaskForm.svelte";
  import { m } from "$lib/paraglide/messages";

  // Set when the task is created from a project or client card.
  const projectId = page.url.searchParams.get("project") ?? undefined;
  const clientId = page.url.searchParams.get("client") ?? undefined;
</script>

<div class="flex flex-col gap-6 p-6">
  <h1 class="text-xl font-semibold text-fg">{m.tasks_new()}</h1>
  <TaskForm
    {projectId}
    {clientId}
    onsaved={(t) => goto(resolve("/tasks/[id]", { id: t.id }))}
    oncancel={() => history.back()}
  />
</div>
