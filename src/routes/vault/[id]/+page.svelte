<script lang="ts">
  import { page } from "$app/state";
  import SecretView from "$lib/features/secrets/SecretView.svelte";
  import VaultScreen from "$lib/features/vault/VaultScreen.svelte";
</script>

<div class="p-6">
  <VaultScreen>
    {#key page.params.id}
      <SecretView id={page.params.id!} />
    {/key}
  </VaultScreen>
</div>
