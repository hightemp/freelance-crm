<script lang="ts">
  import type { Locale, Theme } from "$lib/api/bindings";
  import { i18n } from "$lib/i18n/locale.svelte";
  import { m } from "$lib/paraglide/messages";
  import RadioGroup from "$lib/ui/RadioGroup.svelte";
  import Select from "$lib/ui/Select.svelte";
  import { COMMON_CURRENCIES, currencyLabel } from "./currencies";
  import { appSettings, updateSettings } from "./settings.svelte";
  import { ZOOM_LEVELS } from "./zoom.svelte";

  let s = $derived(appSettings.value);

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
</div>
