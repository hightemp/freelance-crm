<script lang="ts">
  import { commands } from "$lib/api/bindings";
  import { AppError, call } from "$lib/api/client";
  import { codeMessage, errorMessage } from "$lib/i18n/errors";
  import { m } from "$lib/paraglide/messages";
  import Button from "$lib/ui/Button.svelte";
  import Dialog from "$lib/ui/Dialog.svelte";
  import { confirm } from "$lib/ui/dialogs.svelte";
  import TextField from "$lib/ui/TextField.svelte";
  import { toast } from "$lib/ui/toast.svelte";
  import RecoveryKeySetup from "./RecoveryKeySetup.svelte";
  import { refreshVault, vault, watchVault } from "./vault.svelte";

  const MIN_LENGTH = 8;
  let current = $state("");
  let next = $state("");
  let repeat = $state("");
  let errors = $state<{ current?: string; next?: string; repeat?: string }>({});
  let busy = $state(false);
  /** A freshly issued recovery key, shown until confirmed. */
  let newKey = $state<string | null>(null);

  const showError = (err: unknown) => toast.show({ kind: "error", message: errorMessage(err) });

  $effect(() => {
    refreshVault().catch(showError);
    return watchVault();
  });

  async function changePassword(event: SubmitEvent) {
    event.preventDefault();
    errors = {};
    if ([...next].length < MIN_LENGTH) {
      errors.next = codeMessage("validation.min_length", { min: MIN_LENGTH });
    } else if (next !== repeat) {
      errors.repeat = codeMessage("validation.mismatch");
    }
    if (errors.next || errors.repeat) return;
    busy = true;
    try {
      await call(commands.vaultChangePassword(current, next));
      current = next = repeat = "";
      toast.show({ message: m.vault_password_changed_toast() });
    } catch (err) {
      if (err instanceof AppError && err.code === "vault.wrong_password") {
        errors.current = errorMessage(err);
      } else {
        showError(err);
      }
    } finally {
      busy = false;
    }
  }

  async function rotate() {
    const ok = await confirm({
      title: m.vault_rotate_confirm_title(),
      message: m.vault_rotate_confirm_message(),
      confirmLabel: m.vault_rotate_confirm(),
      danger: true,
    });
    if (!ok) return;
    try {
      newKey = await call(commands.vaultRotateRecoveryKey());
    } catch (err) {
      showError(err);
    }
  }
</script>

{#if vault.status !== "unlocked"}
  <p class="text-sm text-muted">{m.vault_settings_locked()}</p>
{:else}
  <div class="flex max-w-md flex-col gap-6">
    <form class="flex flex-col gap-4" novalidate onsubmit={changePassword}>
      <TextField
        label={m.vault_settings_current_password()}
        type="password"
        autocomplete="current-password"
        bind:value={current}
        error={errors.current}
      />
      <TextField
        label={m.vault_settings_new_password()}
        type="password"
        autocomplete="new-password"
        bind:value={next}
        error={errors.next}
      />
      <TextField
        label={m.vault_settings_repeat_password()}
        type="password"
        autocomplete="new-password"
        bind:value={repeat}
        error={errors.repeat}
      />
      <div>
        <Button type="submit" loading={busy}>{m.vault_settings_change_password()}</Button>
      </div>
    </form>
    <div>
      <Button onclick={rotate}>{m.vault_rotate_recovery()}</Button>
    </div>
  </div>
{/if}

{#if newKey}
  <Dialog open title={m.recovery_title()} onclose={() => (newKey = null)}>
    <div class="w-[36rem] max-w-full">
      <RecoveryKeySetup recoveryKey={newKey} oncomplete={() => (newKey = null)} />
    </div>
  </Dialog>
{/if}
