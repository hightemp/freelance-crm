<script lang="ts">
  import Plus from "@lucide/svelte/icons/plus";
  import { goto } from "$app/navigation";
  import { resolve } from "$app/paths";
  import SecretList from "$lib/features/secrets/SecretList.svelte";
  import VaultScreen from "$lib/features/vault/VaultScreen.svelte";
  import { m } from "$lib/paraglide/messages";
  import Button from "$lib/ui/Button.svelte";
</script>

<div class="flex flex-col gap-6 p-6">
  <h1 class="text-xl font-semibold text-fg">{m.nav_vault()}</h1>
  <VaultScreen>
    <div>
      <Button variant="primary" onclick={() => goto(resolve("/vault/new"))}>
        <Plus class="size-4" aria-hidden="true" />
        {m.secrets_new()}
      </Button>
    </div>
    <SecretList />
  </VaultScreen>
</div>
