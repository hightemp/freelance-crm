<script lang="ts">
  import InvoicesPage from "$lib/features/finance/InvoicesPage.svelte";
  import { m } from "$lib/paraglide/messages";
</script>

<div class="flex flex-col gap-6 p-6">
  <h1 class="text-xl font-semibold text-fg">{m.nav_finance()}</h1>
  <InvoicesPage />
</div>
