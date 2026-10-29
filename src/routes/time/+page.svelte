<script lang="ts">
  import TimePage from "$lib/features/time/TimePage.svelte";
  import { m } from "$lib/paraglide/messages";
</script>

<div class="flex flex-col gap-6 p-6">
  <h1 class="text-xl font-semibold text-fg">{m.nav_time()}</h1>
  <TimePage />
</div>
