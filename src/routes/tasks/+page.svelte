<script lang="ts">
  import KanbanBoard from "$lib/features/tasks/KanbanBoard.svelte";
  import TaskList from "$lib/features/tasks/TaskList.svelte";
  import { m } from "$lib/paraglide/messages";
  import Tabs from "$lib/ui/Tabs.svelte";

  let view = $state("list");
  let views = $derived([
    { id: "list", label: m.tasks_view_list() },
    { id: "board", label: m.tasks_view_board() },
  ]);
</script>

<!-- Full height so the list and the board scroll inside the page. -->
<div class="flex h-full flex-col gap-4 p-6">
  <h1 class="text-xl font-semibold text-fg">{m.nav_tasks()}</h1>
  <div class="min-h-0 flex-1">
    <Tabs label={m.tasks_view()} tabs={views} fill bind:value={view}>
      {#snippet panel(current)}
        <div class="h-full min-h-0">
          {#if current === "board"}
            <KanbanBoard />
          {:else}
            <TaskList />
          {/if}
        </div>
      {/snippet}
    </Tabs>
  </div>
</div>
