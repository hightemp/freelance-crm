<script lang="ts">
  import { goto } from "$app/navigation";
  import { resolve } from "$app/paths";
  import { page } from "$app/state";
  import SecretForm from "$lib/features/secrets/SecretForm.svelte";
  import VaultScreen from "$lib/features/vault/VaultScreen.svelte";
  import { m } from "$lib/paraglide/messages";

  // Set when the secret is created from a client or project card.
  const clientId = page.url.searchParams.get("client") ?? undefined;
  const projectId = page.url.searchParams.get("project") ?? undefined;
</script>

<div class="flex flex-col gap-6 p-6">
  <h1 class="text-xl font-semibold text-fg">{m.secrets_new()}</h1>
  <VaultScreen>
    <SecretForm
      {clientId}
      {projectId}
      onsaved={(s) => goto(resolve("/vault/[id]", { id: s.id }))}
      oncancel={() => history.back()}
    />
  </VaultScreen>
</div>
