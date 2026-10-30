<script lang="ts">
  import SettingsForm from "$lib/app/SettingsForm.svelte";
  import BackupSettings from "$lib/features/backup/BackupSettings.svelte";
  import RequisitesForm from "$lib/features/requisites/RequisitesForm.svelte";
  import DocumentSettings from "$lib/features/finance/DocumentSettings.svelte";
  import TaxSettings from "$lib/features/finance/TaxSettings.svelte";
  import VaultSettings from "$lib/features/vault/VaultSettings.svelte";
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
  <section class="flex flex-col gap-4">
    <h2 class="text-base font-semibold text-fg">{m.settings_documents()}</h2>
    <DocumentSettings />
  </section>
  <section class="flex flex-col gap-4">
    <h2 class="text-base font-semibold text-fg">{m.taxes_title()}</h2>
    <TaxSettings />
  </section>
  <section class="flex flex-col gap-4">
    <h2 class="text-base font-semibold text-fg">{m.vault_title()}</h2>
    <VaultSettings />
  </section>
  <section class="flex flex-col gap-4">
    <h2 class="text-base font-semibold text-fg">{m.settings_backups()}</h2>
    <BackupSettings />
  </section>
</div>
