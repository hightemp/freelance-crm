<script lang="ts">
  import { goto } from "$app/navigation";
  import { resolve } from "$app/paths";
  import { page } from "$app/state";
  import ProjectEdit from "$lib/features/projects/ProjectEdit.svelte";

  let id = $derived(page.params.id!);
  const open = (projectId: string) => goto(resolve("/projects/[id]", { id: projectId }));
</script>

<div class="p-6">
  {#key id}
    <ProjectEdit {id} onsaved={(p) => open(p.id)} oncancel={() => open(id)} />
  {/key}
</div>
