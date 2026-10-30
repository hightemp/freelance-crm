<script lang="ts">
  import HomePage from "$lib/features/home/HomePage.svelte";
  import { m } from "$lib/paraglide/messages";
</script>

<div class="flex flex-col gap-6 p-6">
  <h1 class="text-xl font-semibold text-fg">{m.nav_home()}</h1>
  <HomePage />
</div>
