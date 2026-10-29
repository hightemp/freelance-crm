<script lang="ts">
  import TaskList from "$lib/features/tasks/TaskList.svelte";
  import { m } from "$lib/paraglide/messages";
</script>

<!-- Full height so the virtualized list scrolls inside the page. -->
<div class="flex h-full flex-col gap-6 p-6">
  <h1 class="text-xl font-semibold text-fg">{m.nav_tasks()}</h1>
  <div class="min-h-0 flex-1">
    <TaskList />
  </div>
</div>
