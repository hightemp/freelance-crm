<script lang="ts">
  import { commands, type KeyKind } from "$lib/api/bindings";
  import { call } from "$lib/api/client";
  import { errorMessage } from "$lib/i18n/errors";
  import { m } from "$lib/paraglide/messages";
  import Button from "$lib/ui/Button.svelte";
  import Dialog from "$lib/ui/Dialog.svelte";
  import TextField from "$lib/ui/TextField.svelte";
  import { toast } from "$lib/ui/toast.svelte";

  let {
    path,
    kind,
    onrestored,
    onclose,
  }: {
    path: string;
    /** What opens the file: its passphrase or the master password. */
    kind: KeyKind;
    onrestored: () => void;
    onclose: () => void;
  } = $props();

  let secret = $state("");
  let error = $state<string | null>(null);
  let restoring = $state(false);
  let fileName = $derived(path.split(/[\\/]/).pop() ?? path);

  async function submit(event: SubmitEvent) {
    event.preventDefault();
    if (!secret) return;
    restoring = true;
    error = null;
    try {
      await call(commands.restoreBackup(path, secret));
      toast.show({ kind: "success", message: m.backup_restored() });
      onrestored();
    } catch (err) {
      error = errorMessage(err);
    } finally {
      restoring = false;
    }
  }
</script>

<Dialog open role="alertdialog" title={m.backup_restore_title()} {onclose}>
  <form id="backup-restore" class="flex w-[28rem] max-w-full flex-col gap-3" onsubmit={submit}>
    <p class="text-sm text-fg">{m.backup_restore_warning({ file: fileName })}</p>
    <p class="text-sm text-muted">{m.backup_restore_safety()}</p>
    <TextField
      label={kind === "vault" ? m.backup_master_password() : m.backup_password()}
      type="password"
      autocomplete="current-password"
      required
      error={error ?? undefined}
      bind:value={secret}
    />
  </form>
  {#snippet footer()}
    <Button onclick={onclose}>{m.ui_cancel()}</Button>
    <Button
      type="submit"
      form="backup-restore"
      variant="danger"
      loading={restoring}
      disabled={!secret}>{m.backup_restore_confirm()}</Button
    >
  {/snippet}
</Dialog>
