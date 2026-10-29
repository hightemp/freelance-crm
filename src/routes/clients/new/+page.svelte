<script lang="ts">
  import { goto } from "$app/navigation";
  import { resolve } from "$app/paths";
  import ClientForm from "$lib/features/clients/ClientForm.svelte";
  import { m } from "$lib/paraglide/messages";

  const back = () => goto(resolve("/clients"));
</script>

<div class="flex flex-col gap-6 p-6">
  <h1 class="text-xl font-semibold text-fg">{m.clients_new()}</h1>
  <ClientForm onsaved={(c) => goto(resolve("/clients/[id]", { id: c.id }))} oncancel={back} />
</div>
