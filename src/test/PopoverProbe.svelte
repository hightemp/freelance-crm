<script lang="ts">
  import Button from "$lib/ui/Button.svelte";
  import Popover from "$lib/ui/Popover.svelte";
  import Select from "$lib/ui/Select.svelte";
  import TextField from "$lib/ui/TextField.svelte";

  let status = $state<string | null>(null);
</script>

<Popover label="Filters">
  {#snippet trigger(props)}
    <Button {...props}>Filters</Button>
  {/snippet}
  <TextField label="Only active" />
  <Select
    label="Status"
    options={[
      { value: "open", label: "Open" },
      { value: "done", label: "Done" },
    ]}
    bind:value={status}
  />
</Popover>
