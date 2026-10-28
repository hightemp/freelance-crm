<script lang="ts">
  import SettingsForm from "$lib/app/SettingsForm.svelte";
  import { m } from "$lib/paraglide/messages";
</script>

<div class="flex flex-col gap-6 p-6">
  <h1 class="text-xl font-semibold text-fg">{m.nav_settings()}</h1>
  <SettingsForm />
</div>
