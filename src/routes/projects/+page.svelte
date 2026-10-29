<script lang="ts">
  import Plus from "@lucide/svelte/icons/plus";
  import { goto } from "$app/navigation";
  import { resolve } from "$app/paths";
  import ProjectList from "$lib/features/projects/ProjectList.svelte";
  import { m } from "$lib/paraglide/messages";
  import Button from "$lib/ui/Button.svelte";
</script>

<div class="flex flex-col gap-6 p-6">
  <div class="flex items-center justify-between gap-4">
    <h1 class="text-xl font-semibold text-fg">{m.nav_projects()}</h1>
    <Button variant="primary" onclick={() => goto(resolve("/projects/new"))}>
      <Plus class="size-4" aria-hidden="true" />
      {m.projects_new()}
    </Button>
  </div>
  <ProjectList />
</div>
