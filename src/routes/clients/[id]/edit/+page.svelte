<script lang="ts">
  import { goto } from "$app/navigation";
  import { resolve } from "$app/paths";
  import { page } from "$app/state";
  import ClientEdit from "$lib/features/clients/ClientEdit.svelte";

  let id = $derived(page.params.id!);
  const open = (clientId: string) => goto(resolve("/clients/[id]", { id: clientId }));
</script>

<div class="p-6">
  {#key id}
    <ClientEdit {id} onsaved={(c) => open(c.id)} oncancel={() => open(id)} />
  {/key}
</div>
