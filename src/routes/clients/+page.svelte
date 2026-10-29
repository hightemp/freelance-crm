<script lang="ts">
  import Plus from "@lucide/svelte/icons/plus";
  import { goto } from "$app/navigation";
  import { resolve } from "$app/paths";
  import ClientList from "$lib/features/clients/ClientList.svelte";
  import { m } from "$lib/paraglide/messages";
  import Button from "$lib/ui/Button.svelte";
</script>

<div class="flex flex-col gap-6 p-6">
  <div class="flex items-center justify-between gap-4">
    <h1 class="text-xl font-semibold text-fg">{m.nav_clients()}</h1>
    <Button variant="primary" onclick={() => goto(resolve("/clients/new"))}>
      <Plus class="size-4" aria-hidden="true" />
      {m.clients_new()}
    </Button>
  </div>
  <ClientList />
</div>
