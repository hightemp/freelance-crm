<script lang="ts">
  import { commands } from "$lib/api/bindings";
  import { AppError, call } from "$lib/api/client";
  import { codeMessage, errorMessage } from "$lib/i18n/errors";
  import { m } from "$lib/paraglide/messages";
  import Button from "$lib/ui/Button.svelte";
  import TextArea from "$lib/ui/TextArea.svelte";
  import TextField from "$lib/ui/TextField.svelte";

  let { onrecovered, onback }: { onrecovered: () => void; onback: () => void } = $props();

  const MIN_LENGTH = 8;
  let recoveryKey = $state("");
  let password = $state("");
  let confirm = $state("");
  let errors = $state<{ key?: string; password?: string; confirm?: string }>({});
  let busy = $state(false);

  async function recover(event: SubmitEvent) {
    event.preventDefault();
    errors = {};
    if (!recoveryKey.trim()) errors.key = codeMessage("validation.required");
    if ([...password].length < MIN_LENGTH) {
      errors.password = codeMessage("validation.min_length", { min: MIN_LENGTH });
    } else if (password !== confirm) {
      errors.confirm = codeMessage("validation.mismatch");
    }
    if (errors.key || errors.password || errors.confirm) return;
    busy = true;
    try {
      await call(commands.vaultRecover(recoveryKey, password));
      recoveryKey = password = confirm = "";
      onrecovered();
    } catch (err) {
      if (err instanceof AppError && err.params.field === "password") {
        errors.password = errorMessage(err);
      } else {
        errors.key = errorMessage(err);
      }
    } finally {
      busy = false;
    }
  }
</script>

<form class="flex max-w-md flex-col gap-4" novalidate onsubmit={recover}>
  <h2 class="text-base font-semibold text-fg">{m.recover_title()}</h2>
  <p class="text-sm text-muted">{m.recover_hint()}</p>
  <TextArea
    label={m.recover_key()}
    rows={3}
    spellcheck="false"
    autocomplete="off"
    class="font-mono"
    bind:value={recoveryKey}
    error={errors.key}
  />
  <TextField
    label={m.vault_settings_new_password()}
    type="password"
    autocomplete="new-password"
    bind:value={password}
    error={errors.password}
  />
  <TextField
    label={m.vault_settings_repeat_password()}
    type="password"
    autocomplete="new-password"
    bind:value={confirm}
    error={errors.confirm}
  />
  <div class="flex gap-2">
    <Button type="submit" variant="primary" loading={busy}>{m.recover_submit()}</Button>
    <Button onclick={onback}>{m.recover_back()}</Button>
  </div>
</form>
