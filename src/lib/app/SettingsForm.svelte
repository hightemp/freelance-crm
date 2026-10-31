<script lang="ts">
  import { commands, type Locale, type Theme } from "$lib/api/bindings";
  import { call } from "$lib/api/client";
  import { checkForUpdate, updates } from "$lib/features/updates/updates.svelte";
  import { errorMessage } from "$lib/i18n/errors";
  import { i18n } from "$lib/i18n/locale.svelte";
  import { m } from "$lib/paraglide/messages";
  import Button from "$lib/ui/Button.svelte";
  import RadioGroup from "$lib/ui/RadioGroup.svelte";
  import Select from "$lib/ui/Select.svelte";
  import Switch from "$lib/ui/Switch.svelte";
  import { toast } from "$lib/ui/toast.svelte";
  import { COMMON_CURRENCIES, currencyLabel } from "./currencies";
  import { appSettings, updateSettings } from "./settings.svelte";
  import { ZOOM_LEVELS } from "./zoom.svelte";

  let s = $derived(appSettings.value);
  /** Launch at login lives in the OS, not in the settings. */
  let autostart = $state(false);

  let version = $state("");
  let checking = $state(false);
  /** The result of the last check, shown under the button. */
  let checkResult = $state("");

  $effect(() => {
    call(commands.autostartEnabled())
      .then((on) => (autostart = on))
      .catch(() => {});
    call(commands.appInfo())
      .then((info) => (version = info.version))
      .catch(() => {});
  });

  async function checkNow() {
    checking = true;
    try {
      const update = await checkForUpdate();
      checkResult = update ? m.update_dialog_title({ version: update.version }) : m.update_latest();
      if (update) updates.dialogOpen = true;
    } catch (err) {
      checkResult = "";
      toast.show({ kind: "error", message: errorMessage(err) });
    } finally {
      checking = false;
    }
  }

  async function setAutostart(on: boolean) {
    const before = autostart;
    autostart = on;
    try {
      await call(commands.setAutostart(on));
    } catch (err) {
      autostart = before;
      toast.show({ kind: "error", message: errorMessage(err) });
    }
  }

  const weekdayName = (day: number) =>
    new Intl.DateTimeFormat(i18n.locale, { weekday: "long", timeZone: "UTC" }).format(
      Date.UTC(2023, 0, 1 + day), // 2023-01-01 is a Sunday
    );

  let languages = $derived([
    { value: "system", label: m.settings_language_system() },
    // Language names are shown in their own language.
    { value: "en", label: "English" },
    { value: "ru", label: "Русский" },
  ]);
  let currencies = $derived(
    COMMON_CURRENCIES.map((code) => ({ value: code, label: currencyLabel(code, i18n.locale) })),
  );
  let weekStarts = $derived([1, 0, 6].map((d) => ({ value: String(d), label: weekdayName(d) })));
  const AUTOLOCK_MINUTES = [1, 5, 15, 30, 60, 0];
  let autolocks = $derived(
    AUTOLOCK_MINUTES.map((minutes) => ({
      value: String(minutes),
      label: minutes === 0 ? m.autolock_never() : m.autolock_minutes({ count: minutes }),
    })),
  );
  const zooms = ZOOM_LEVELS.map((z) => ({
    value: String(Math.round(z * 100)),
    label: `${Math.round(z * 100)}%`,
  }));
</script>

<div class="flex max-w-md flex-col gap-8">
  <section class="flex flex-col gap-4">
    <h2 class="text-base font-semibold text-fg">{m.settings_regional()}</h2>
    <Select
      label={m.settings_language()}
      options={languages}
      bind:value={
        () => s.locale ?? "system",
        (v) => updateSettings({ locale: v === "system" ? null : (v as Locale) })
      }
    />
    <Select
      label={m.settings_currency()}
      options={currencies}
      bind:value={() => s.defaultCurrency, (v) => v && updateSettings({ defaultCurrency: v })}
    />
    <Select
      label={m.settings_week_start()}
      options={weekStarts}
      bind:value={() => String(s.weekStart), (v) => updateSettings({ weekStart: Number(v) })}
    />
  </section>

  <section class="flex flex-col gap-4">
    <h2 class="text-base font-semibold text-fg">{m.settings_appearance()}</h2>
    <RadioGroup
      label={m.ui_theme()}
      options={[
        { value: "light", label: m.theme_light() },
        { value: "dark", label: m.theme_dark() },
        { value: "system", label: m.theme_system() },
      ]}
      bind:value={() => s.theme, (v) => updateSettings({ theme: v as Theme })}
    />
    <Select
      label={m.settings_zoom()}
      options={zooms}
      bind:value={() => String(s.zoomPercent), (v) => updateSettings({ zoomPercent: Number(v) })}
    />
  </section>

  <section class="flex flex-col gap-4">
    <h2 class="text-base font-semibold text-fg">{m.settings_system()}</h2>
    <Switch
      label={m.settings_close_to_tray()}
      bind:checked={() => s.closeToTray, (on) => void updateSettings({ closeToTray: on })}
    />
    <Switch
      label={m.settings_autostart()}
      bind:checked={() => autostart, (on) => void setAutostart(on)}
    />
  </section>

  <section class="flex flex-col gap-4">
    <h2 class="text-base font-semibold text-fg">{m.settings_updates()}</h2>
    {#if version}
      <p class="text-sm text-muted">{m.settings_version({ version })}</p>
    {/if}
    <Switch
      label={m.settings_check_updates()}
      bind:checked={() => s.checkUpdates, (on) => void updateSettings({ checkUpdates: on })}
    />
    <div class="flex flex-wrap items-center gap-3">
      <Button loading={checking} onclick={checkNow}>{m.settings_check_now()}</Button>
      <p class="text-sm text-muted" aria-live="polite">{checkResult}</p>
    </div>
  </section>

  <section class="flex flex-col gap-4">
    <h2 class="text-base font-semibold text-fg">{m.settings_security()}</h2>
    <Select
      label={m.settings_autolock()}
      options={autolocks}
      bind:value={
        () => String(s.vaultAutolockMinutes),
        (v) => updateSettings({ vaultAutolockMinutes: Number(v) })
      }
    />
  </section>
</div>
