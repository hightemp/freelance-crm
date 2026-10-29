<script lang="ts">
  import { commands, type Project_Serialize as Project } from "$lib/api/bindings";
  import { call } from "$lib/api/client";
  import { errorMessage } from "$lib/i18n/errors";
  import { m } from "$lib/paraglide/messages";
  import Spinner from "$lib/ui/Spinner.svelte";
  import ProjectForm from "./ProjectForm.svelte";

  let {
    id,
    onsaved,
    oncancel,
  }: { id: string; onsaved: (project: Project) => void; oncancel: () => void } = $props();

  let project = $state<Project | null>(null);
  let loadError = $state<string | null>(null);

  $effect(() => {
    call(commands.getProject(id)).then(
      (p) => (project = p),
      (err) => (loadError = errorMessage(err)),
    );
  });
</script>

<div class="flex flex-col gap-6">
  <h1 class="text-xl font-semibold text-fg">{m.project_edit()}</h1>
  {#if loadError}
    <p class="text-sm text-danger" role="alert">{loadError}</p>
  {:else if project}
    <ProjectForm {project} {onsaved} {oncancel} />
  {:else}
    <div class="flex justify-center p-8"><Spinner /></div>
  {/if}
</div>
