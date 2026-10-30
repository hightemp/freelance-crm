<script lang="ts">
  import { save } from "@tauri-apps/plugin-dialog";
  import { commands } from "$lib/api/bindings";
  import { call } from "$lib/api/client";
  import { codeMessage, errorMessage } from "$lib/i18n/errors";
  import { m } from "$lib/paraglide/messages";
  import Button from "$lib/ui/Button.svelte";
  import { todayIso } from "$lib/ui/date";
  import Dialog from "$lib/ui/Dialog.svelte";
  import TextField from "$lib/ui/TextField.svelte";
  import { toast } from "$lib/ui/toast.svelte";

  let { onclose }: { onclose: () => void } = $props();

  const MIN = 8;
  let password = $state("");
  let repeat = $state("");
  let error = $state<{ field: "password" | "repeat"; message: string } | null>(null);
  let saving = $state(false);

  async function submit(event: SubmitEvent) {
    event.preventDefault();
    if ([...password].length < MIN) {
      error = { field: "password", message: codeMessage("validation.min_length", { min: MIN }) };
      return;
    }
    if (password !== repeat) {
      error = { field: "repeat", message: codeMessage("validation.mismatch") };
      return;
    }
    error = null;
    const path = await save({
      defaultPath: `freelance-crm-${todayIso()}.fcrm-backup`,
      filters: [{ name: "Freelance CRM", extensions: ["fcrm-backup"] }],
    }).catch(() => null);
    if (!path) return;
    saving = true;
    try {
      await call(commands.exportBackup(path, password));
      toast.show({ kind: "success", message: m.backup_saved() });
      onclose();
    } catch (err) {
      toast.show({ kind: "error", message: errorMessage(err) });
    } finally {
      saving = false;
    }
  }
</script>

<Dialog open title={m.backup_create_title()} description={m.backup_create_hint()} {onclose}>
  <form
    id="backup-export"
    class="flex w-[26rem] max-w-full flex-col gap-3"
    novalidate
    onsubmit={submit}
  >
    <TextField
      label={m.backup_password()}
      type="password"
      autocomplete="new-password"
      required
      error={error?.field === "password" ? error.message : undefined}
      bind:value={password}
    />
    <TextField
      label={m.backup_password_repeat()}
      type="password"
      autocomplete="new-password"
      required
      error={error?.field === "repeat" ? error.message : undefined}
      bind:value={repeat}
    />
  </form>
  {#snippet footer()}
    <Button onclick={onclose}>{m.ui_cancel()}</Button>
    <Button type="submit" form="backup-export" variant="primary" loading={saving}
      >{m.backup_choose_and_save()}</Button
    >
  {/snippet}
</Dialog>
