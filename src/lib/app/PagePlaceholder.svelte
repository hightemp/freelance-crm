<script lang="ts">
  import { m } from "$lib/paraglide/messages";

  let { title }: { title: string } = $props();
</script>

<div class="flex flex-col gap-2 p-6">
  <h1 class="text-xl font-semibold text-fg">{title}</h1>
  <p class="text-sm text-muted">{m.page_coming_soon()}</p>
</div>
