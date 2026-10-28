<script lang="ts">
  import Button from "$lib/ui/Button.svelte";
  import Popover from "$lib/ui/Popover.svelte";
  import TextField from "$lib/ui/TextField.svelte";
</script>

<Popover label="Filters">
  {#snippet trigger(props)}
    <Button {...props}>Filters</Button>
  {/snippet}
  <TextField label="Only active" />
</Popover>
