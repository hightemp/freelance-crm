<script lang="ts">
  import { m } from "$lib/paraglide/messages";
  import Button from "./Button.svelte";
  import Dialog from "./Dialog.svelte";
  import TextField from "./TextField.svelte";
  import { currentDialog, settleDialog } from "./dialogs.svelte";

  let request = $derived(currentDialog());
  let promptValue = $state("");

  $effect.pre(() => {
    if (request?.kind === "prompt") promptValue = request.options.initial ?? "";
  });
</script>

{#if request}
  {#key request}
    {#if request.kind === "confirm"}
      {@const o = request.options}
      <Dialog
        open
        role="alertdialog"
        title={o.title}
        description={o.message}
        onclose={() => settleDialog(false)}
      >
        {#snippet footer()}
          <Button data-autofocus={o.danger || undefined} onclick={() => settleDialog(false)}>
            {o.cancelLabel ?? m.ui_cancel()}
          </Button>
          <Button
            variant={o.danger ? "danger" : "primary"}
            data-autofocus={!o.danger || undefined}
            onclick={() => settleDialog(true)}
          >
            {o.confirmLabel ?? m.ui_ok()}
          </Button>
        {/snippet}
      </Dialog>
    {:else}
      {@const o = request.options}
      <Dialog open title={o.title} description={o.message} onclose={() => settleDialog(null)}>
        <form
          id="prompt-form"
          novalidate
          onsubmit={(e) => {
            e.preventDefault();
            settleDialog(promptValue);
          }}
        >
          <TextField label={o.label} bind:value={promptValue} data-autofocus />
        </form>
        {#snippet footer()}
          <Button onclick={() => settleDialog(null)}>{o.cancelLabel ?? m.ui_cancel()}</Button>
          <Button type="submit" form="prompt-form" variant="primary">
            {o.confirmLabel ?? m.ui_ok()}
          </Button>
        {/snippet}
      </Dialog>
    {/if}
  {/key}
{/if}
