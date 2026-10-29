<script lang="ts">
  import VaultScreen from "$lib/features/vault/VaultScreen.svelte";
  import { m } from "$lib/paraglide/messages";
</script>

<div class="flex flex-col gap-6 p-6">
  <h1 class="text-xl font-semibold text-fg">{m.nav_vault()}</h1>
  <VaultScreen />
</div>
