<script lang="ts">
  import Button from "$lib/ui/Button.svelte";
  import ContextMenu from "$lib/ui/ContextMenu.svelte";
  import DropdownMenu from "$lib/ui/DropdownMenu.svelte";
  import type { MenuEntry } from "$lib/ui/menu";

  let { onSelect }: { onSelect: (id: string) => void } = $props();

  const items: MenuEntry[] = [
    { id: "edit", label: "Edit" },
    { id: "duplicate", label: "Duplicate" },
    { id: "archive", label: "Archive", disabled: true },
    { separator: true },
    { id: "delete", label: "Delete", danger: true },
  ];
</script>

<DropdownMenu {items} {onSelect}>
  {#snippet trigger(props)}
    <Button {...props}>Actions</Button>
  {/snippet}
</DropdownMenu>

<ContextMenu {items} {onSelect}>
  <button type="button" data-testid="context-area">Right-click me</button>
</ContextMenu>
