<script lang="ts">
  import Lock from "@lucide/svelte/icons/lock";
  import type { Snippet } from "svelte";
  import { errorMessage } from "$lib/i18n/errors";
  import { m } from "$lib/paraglide/messages";
  import Button from "$lib/ui/Button.svelte";
  import Spinner from "$lib/ui/Spinner.svelte";
  import { toast } from "$lib/ui/toast.svelte";
  import CreateVault from "./CreateVault.svelte";
  import RecoverVault from "./RecoverVault.svelte";
  import RecoveryKeySetup from "./RecoveryKeySetup.svelte";
  import UnlockVault from "./UnlockVault.svelte";
  import { lockVault, refreshVault, vault, watchVault } from "./vault.svelte";

  // Content for an unlocked vault (the secrets list); a short note by default.
  let { children }: { children?: Snippet } = $props();

  const showError = (err: unknown) => toast.show({ kind: "error", message: errorMessage(err) });

  $effect(() => {
    refreshVault().catch(showError);
    return watchVault();
  });

  const reload = () => refreshVault().catch(showError);

  /** Shown once right after the vault is created. */
  let recoveryKey = $state<string | null>(null);
  let recovering = $state(false);
</script>

{#if recoveryKey}
  <RecoveryKeySetup
    {recoveryKey}
    oncomplete={() => {
      recoveryKey = null;
      void reload();
    }}
  />
{:else if vault.status === null}
  <div class="flex justify-center p-8"><Spinner /></div>
{:else if vault.status === "uninitialized"}
  <CreateVault oncreated={(key) => (recoveryKey = key)} />
{:else if vault.status === "locked" && recovering}
  <RecoverVault
    onrecovered={() => {
      recovering = false;
      void reload();
    }}
    onback={() => (recovering = false)}
  />
{:else if vault.status === "locked"}
  <UnlockVault onunlocked={reload} onforgot={() => (recovering = true)} />
{:else}
  <div class="flex flex-col gap-4">
    <div>
      <Button onclick={() => lockVault().catch(showError)}>
        <Lock class="size-4" aria-hidden="true" />
        {m.vault_lock()}
      </Button>
    </div>
    {#if children}
      {@render children()}
    {:else}
      <p class="text-sm text-muted">{m.vault_unlocked_hint()}</p>
    {/if}
  </div>
{/if}
