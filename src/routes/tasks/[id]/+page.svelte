<script lang="ts">
  import { page } from "$app/state";
  import TaskCard from "$lib/features/tasks/TaskCard.svelte";
</script>

<div class="p-6">
  {#key page.params.id}
    <TaskCard id={page.params.id!} />
  {/key}
</div>
