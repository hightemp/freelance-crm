<script lang="ts">
  import { page } from "$app/state";
  import ClientCard from "$lib/features/clients/ClientCard.svelte";
</script>

<div class="p-6">
  {#key page.params.id}
    <ClientCard id={page.params.id!} initialTab={page.url.searchParams.get("tab") ?? undefined} />
  {/key}
</div>
