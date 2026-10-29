<script lang="ts">
  import { goto } from "$app/navigation";
  import { resolve } from "$app/paths";
  import { page } from "$app/state";
  import TaskEdit from "$lib/features/tasks/TaskEdit.svelte";

  let id = $derived(page.params.id!);
  const open = (taskId: string) => goto(resolve("/tasks/[id]", { id: taskId }));
</script>

<div class="p-6">
  {#key id}
    <TaskEdit {id} onsaved={(t) => open(t.id)} oncancel={() => open(id)} />
  {/key}
</div>
