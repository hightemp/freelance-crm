<script lang="ts">
  import ClientList from "$lib/features/clients/ClientList.svelte";
  import { m } from "$lib/paraglide/messages";
</script>

<div class="flex flex-col gap-6 p-6">
  <h1 class="text-xl font-semibold text-fg">{m.nav_clients()}</h1>
  <ClientList />
</div>
