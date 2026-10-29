<script lang="ts">
  import { goto } from "$app/navigation";
  import { resolve } from "$app/paths";
  import { page } from "$app/state";
  import SecretEdit from "$lib/features/secrets/SecretEdit.svelte";
  import VaultScreen from "$lib/features/vault/VaultScreen.svelte";

  let id = $derived(page.params.id!);
  const open = (secretId: string) => goto(resolve("/vault/[id]", { id: secretId }));
</script>

<div class="p-6">
  <VaultScreen>
    {#key id}
      <SecretEdit {id} onsaved={(s) => open(s.id)} oncancel={() => open(id)} />
    {/key}
  </VaultScreen>
</div>
