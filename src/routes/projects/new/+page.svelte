<script lang="ts">
  import { goto } from "$app/navigation";
  import { resolve } from "$app/paths";
  import { page } from "$app/state";
  import ProjectForm from "$lib/features/projects/ProjectForm.svelte";
  import { m } from "$lib/paraglide/messages";

  // Set when the project is created from a client card.
  const clientId = page.url.searchParams.get("client") ?? undefined;
</script>

<div class="flex flex-col gap-6 p-6">
  <h1 class="text-xl font-semibold text-fg">{m.projects_new()}</h1>
  <ProjectForm
    {clientId}
    onsaved={(p) => goto(resolve("/projects/[id]", { id: p.id }))}
    oncancel={() => history.back()}
  />
</div>
