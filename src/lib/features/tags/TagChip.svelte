<script lang="ts">
  let { name, color }: { name: string; color: string } = $props();
</script>

<span
  class="inline-flex max-w-40 items-center gap-1.5 rounded-full border border-border px-2 py-0.5 text-xs text-fg"
>
  <span class="size-2 shrink-0 rounded-full" style:background-color={color} aria-hidden="true"
  ></span>
  <span class="truncate">{name}</span>
</span>
