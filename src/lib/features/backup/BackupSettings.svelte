<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import {
    commands,
    type AutoBackupInfo_Serialize as AutoBackupInfo,
    type BackupKeySource,
    type BackupSchedule,
    type KeyKind,
  } from "$lib/api/bindings";
  import { call } from "$lib/api/client";
  import { appSettings, updateSettings } from "$lib/app/settings.svelte";
  import { codeMessage, errorMessage } from "$lib/i18n/errors";
  import { formatDate } from "$lib/i18n/format";
  import { i18n } from "$lib/i18n/locale.svelte";
  import { m } from "$lib/paraglide/messages";
  import Button from "$lib/ui/Button.svelte";
  import NumberField from "$lib/ui/NumberField.svelte";
  import RadioGroup from "$lib/ui/RadioGroup.svelte";
  import Select from "$lib/ui/Select.svelte";
  import TextField from "$lib/ui/TextField.svelte";
  import { toast } from "$lib/ui/toast.svelte";
  import ExportBackupDialog from "./ExportBackupDialog.svelte";
  import RestoreBackupDialog from "./RestoreBackupDialog.svelte";

  let { onrestored = () => location.reload() }: { onrestored?: () => void } = $props();

  let exporting = $state(false);
  let restoring = $state<{ path: string; kind: KeyKind } | null>(null);
  let status = $state<AutoBackupInfo | null>(null);
  let keychainPassword = $state("");

  const showError = (err: unknown) => toast.show({ kind: "error", message: errorMessage(err) });
  let settings = $derived(appSettings.value);

  async function loadStatus() {
    try {
      status = await call(commands.autoBackupStatus());
    } catch (err) {
      showError(err);
    }
  }

  $effect(() => {
    void loadStatus();
  });

  async function chooseRestore() {
    const path = await open({
      multiple: false,
      directory: false,
      filters: [{ name: "Freelance CRM", extensions: ["fcrm-backup"] }],
    }).catch(() => null);
    if (!path) return;
    try {
      restoring = { path, kind: await call(commands.inspectBackup(path)) };
    } catch (err) {
      showError(err);
    }
  }

  async function chooseFolder() {
    const dir = await open({ multiple: false, directory: true }).catch(() => null);
    if (dir) await updateSettings({ autoBackupDir: dir });
  }

  async function saveKeychain(passphrase: string | null) {
    try {
      await call(commands.setBackupPassphrase(passphrase));
      keychainPassword = "";
      await loadStatus();
    } catch (err) {
      showError(err);
    }
  }

  async function setUpVault() {
    try {
      await call(commands.setUpVaultBackupKey());
      await loadStatus();
    } catch (err) {
      showError(err);
    }
  }

  async function exportData() {
    const dir = await open({ multiple: false, directory: true }).catch(() => null);
    if (!dir) return;
    try {
      const files = await call(commands.exportData(dir));
      toast.show({ kind: "success", message: m.backup_exported({ count: files.length }) });
    } catch (err) {
      showError(err);
    }
  }
</script>

<div class="flex max-w-2xl flex-col gap-6">
  <div class="flex flex-wrap gap-2">
    <Button onclick={() => (exporting = true)}>{m.backup_create()}</Button>
    <Button onclick={chooseRestore}>{m.backup_restore()}</Button>
    <Button variant="ghost" onclick={exportData}>{m.backup_export_data()}</Button>
  </div>

  <section class="flex flex-col gap-4" aria-labelledby="auto-backup-title">
    <h3 id="auto-backup-title" class="text-sm font-semibold text-fg">{m.backup_auto_title()}</h3>
    <div class="flex items-end gap-2">
      <div class="flex-1">
        <TextField
          label={m.backup_folder()}
          readonly
          placeholder={m.backup_folder_none()}
          value={settings.autoBackupDir ?? ""}
        />
      </div>
      <Button onclick={chooseFolder}>{m.backup_choose_folder()}</Button>
    </div>
    <div class="grid gap-3 sm:grid-cols-2">
      <Select
        label={m.backup_schedule()}
        disabled={!settings.autoBackupDir}
        options={[
          { value: "off", label: m.backup_schedule_off() },
          { value: "daily", label: m.backup_schedule_daily() },
          { value: "weekly", label: m.backup_schedule_weekly() },
        ]}
        bind:value={
          () => settings.autoBackupSchedule,
          (v) => void updateSettings({ autoBackupSchedule: (v ?? "off") as BackupSchedule })
        }
      />
      <NumberField
        label={m.backup_keep()}
        min={1}
        max={100}
        bind:value={
          () => settings.autoBackupKeep,
          (v) => v !== null && v >= 1 && v <= 100 && void updateSettings({ autoBackupKeep: v })
        }
      />
    </div>
    <RadioGroup
      label={m.backup_key()}
      options={[
        { value: "vault", label: m.backup_key_vault() },
        { value: "keychain", label: m.backup_key_keychain() },
      ]}
      bind:value={
        () => settings.autoBackupKey,
        (v) => void updateSettings({ autoBackupKey: v as BackupKeySource })
      }
    />
    {#if settings.autoBackupKey === "vault"}
      <div class="flex items-center gap-3 text-sm">
        {#if status?.vaultKeyReady}
          <span class="text-muted">{m.backup_vault_ready()}</span>
        {:else}
          <span class="text-muted">{m.backup_vault_needs_setup()}</span>
          <Button size="sm" onclick={setUpVault}>{m.backup_set_up()}</Button>
        {/if}
      </div>
    {:else}
      <div class="flex items-end gap-2">
        <div class="flex-1">
          <TextField
            label={m.backup_keychain_password()}
            type="password"
            autocomplete="new-password"
            hint={status?.keychainReady ? m.backup_keychain_saved() : m.backup_keychain_none()}
            bind:value={keychainPassword}
          />
        </div>
        <Button disabled={!keychainPassword} onclick={() => saveKeychain(keychainPassword)}
          >{m.backup_keychain_save()}</Button
        >
        {#if status?.keychainReady}
          <Button variant="ghost" onclick={() => saveKeychain(null)}
            >{m.backup_keychain_remove()}</Button
          >
        {/if}
      </div>
    {/if}
    {#if status}
      <p class="text-sm text-muted" role="status">
        {#if status.lastError}
          {m.backup_last_failed({ error: codeMessage(status.lastError) })}
        {:else if status.lastSuccessAt}
          {m.backup_last_success({ date: formatDate(status.lastSuccessAt, i18n.locale) })}
        {:else}
          {m.backup_never()}
        {/if}
      </p>
    {/if}
  </section>
</div>

{#if exporting}
  <ExportBackupDialog onclose={() => (exporting = false)} />
{/if}
{#if restoring}
  <RestoreBackupDialog
    path={restoring.path}
    kind={restoring.kind}
    {onrestored}
    onclose={() => (restoring = null)}
  />
{/if}
