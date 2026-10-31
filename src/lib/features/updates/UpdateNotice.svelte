<script lang="ts">
  import Download from "@lucide/svelte/icons/download";
  import { errorMessage } from "$lib/i18n/errors";
  import { m } from "$lib/paraglide/messages";
  import Button from "$lib/ui/Button.svelte";
  import Dialog from "$lib/ui/Dialog.svelte";
  import { toast } from "$lib/ui/toast.svelte";
  import { installUpdate, updates } from "./updates.svelte";

  let installing = $state(false);

  async function install() {
    installing = true;
    try {
      await installUpdate();
    } catch (err) {
      toast.show({ kind: "error", message: errorMessage(err) });
    } finally {
      installing = false;
    }
  }
</script>

{#if updates.available}
  {@const version = updates.available.version}
  <Button size="sm" variant="ghost" onclick={() => (updates.dialogOpen = true)}>
    <Download class="size-4" aria-hidden="true" />
    {m.update_notice({ version })}
  </Button>

  <Dialog
    bind:open={updates.dialogOpen}
    title={m.update_dialog_title({ version })}
    description={m.update_dialog_description()}
    dismissable={!installing}
  >
    {#if updates.available.notes}
      <div class="max-h-64 overflow-y-auto text-sm whitespace-pre-wrap text-muted">
        {updates.available.notes}
      </div>
    {/if}
    {#snippet footer()}
      <Button disabled={installing} onclick={() => (updates.dialogOpen = false)}>
        {m.update_later()}
      </Button>
      <Button variant="primary" loading={installing} onclick={install}>
        {m.update_install()}
      </Button>
    {/snippet}
  </Dialog>
{/if}
