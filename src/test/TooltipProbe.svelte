<script lang="ts">
  import Button from "$lib/ui/Button.svelte";
  import Tooltip from "$lib/ui/Tooltip.svelte";
</script>

<Tooltip text="Copy to clipboard">
  {#snippet children(trigger)}
    <Button {...trigger}>Copy</Button>
  {/snippet}
</Tooltip>
