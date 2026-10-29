<script lang="ts">
  import VirtualList from "$lib/ui/VirtualList.svelte";

  let { count }: { count: number } = $props();
  const items = Array.from({ length: count }, (_, i) => ({ id: i, name: `Row ${i}` }));
</script>

<VirtualList label="Rows" {items} key={(item) => item.id} rowHeight={32} height={320} overscan={3}>
  {#snippet row(item)}<span>{item.name}</span>{/snippet}
</VirtualList>
