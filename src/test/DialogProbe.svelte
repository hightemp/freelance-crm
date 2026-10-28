<script lang="ts">
  import Button from "$lib/ui/Button.svelte";
  import Dialog from "$lib/ui/Dialog.svelte";
  import TextField from "$lib/ui/TextField.svelte";

  let open = $state(false);
</script>

<Button onclick={() => (open = true)}>Open</Button>
<Dialog bind:open title="Probe">
  <TextField label="First" />
  {#snippet footer()}
    <Button>Save</Button>
  {/snippet}
</Dialog>
