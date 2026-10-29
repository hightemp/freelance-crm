<script lang="ts">
  import { page } from "$app/state";
  import ProjectCard from "$lib/features/projects/ProjectCard.svelte";
</script>

<div class="p-6">
  {#key page.params.id}
    <ProjectCard id={page.params.id!} />
  {/key}
</div>
