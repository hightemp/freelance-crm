<script lang="ts">
  import SettingsForm from "$lib/app/SettingsForm.svelte";
  import RequisitesForm from "$lib/features/requisites/RequisitesForm.svelte";
  import { m } from "$lib/paraglide/messages";
</script>

<div class="flex flex-col gap-10 p-6">
  <h1 class="text-xl font-semibold text-fg">{m.nav_settings()}</h1>
  <SettingsForm />
  <section class="flex flex-col gap-4">
    <div>
      <h2 class="text-base font-semibold text-fg">{m.settings_profile()}</h2>
      <p class="text-sm text-muted">{m.settings_profile_hint()}</p>
    </div>
    <RequisitesForm owner={{ type: "profile" }} withName />
  </section>
</div>
