<script lang="ts">
  import Lock from "@lucide/svelte/icons/lock";
  import LockOpen from "@lucide/svelte/icons/lock-open";
  import { goto } from "$app/navigation";
  import { resolve } from "$app/paths";
  import { errorMessage } from "$lib/i18n/errors";
  import { m } from "$lib/paraglide/messages";
  import Button from "$lib/ui/Button.svelte";
  import { toast } from "$lib/ui/toast.svelte";
  import { lockVault, refreshVault, vault, watchVault } from "./vault.svelte";

  const showError = (err: unknown) => toast.show({ kind: "error", message: errorMessage(err) });

  $effect(() => {
    refreshVault().catch(showError);
    return watchVault();
  });

  let label = $derived(
    vault.status === "unlocked"
      ? m.vault_indicator_unlocked()
      : vault.status === "locked"
        ? m.vault_indicator_locked()
        : m.vault_indicator_uninitialized(),
  );

  function onclick() {
    if (vault.status === "unlocked") lockVault().catch(showError);
    else void goto(resolve("/vault"));
  }
</script>

<Button variant="ghost" size="sm" aria-label={label} {onclick}>
  {#if vault.status === "unlocked"}
    <LockOpen class="size-4 text-accent" aria-hidden="true" />
  {:else}
    <Lock class="size-4" aria-hidden="true" />
  {/if}
</Button>
