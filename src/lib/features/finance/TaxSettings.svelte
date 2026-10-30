<script lang="ts">
  import Pencil from "@lucide/svelte/icons/pencil";
  import Trash2 from "@lucide/svelte/icons/trash-2";
  import {
    commands,
    type ClientKind,
    type TaxRegime,
    type TaxRegimeInput,
    type TaxPresetOption,
  } from "$lib/api/bindings";
  import { call } from "$lib/api/client";
  import { COMMON_CURRENCIES } from "$lib/app/currencies";
  import { appSettings, updateSettings } from "$lib/app/settings.svelte";
  import { errorMessage } from "$lib/i18n/errors";
  import { formatIsoDate } from "$lib/i18n/format";
  import { i18n } from "$lib/i18n/locale.svelte";
  import { m } from "$lib/paraglide/messages";
  import Button from "$lib/ui/Button.svelte";
  import DatePicker from "$lib/ui/DatePicker.svelte";
  import { todayIso } from "$lib/ui/date";
  import Dialog from "$lib/ui/Dialog.svelte";
  import { confirm } from "$lib/ui/dialogs.svelte";
  import NumberField from "$lib/ui/NumberField.svelte";
  import Select from "$lib/ui/Select.svelte";
  import Switch from "$lib/ui/Switch.svelte";
  import TextField from "$lib/ui/TextField.svelte";
  import { toast } from "$lib/ui/toast.svelte";

  let { today = todayIso() }: { today?: string } = $props();

  const ANY = "";
  let regimes = $state<TaxRegime[] | null>(null);
  let presets = $state<TaxPresetOption[]>([]);
  /** The regime being edited: its id (null when new) and draft. */
  let editing = $state<{ id: string | null; draft: TaxRegimeInput } | null>(null);
  let saving = $state(false);

  const showError = (err: unknown) => toast.show({ kind: "error", message: errorMessage(err) });

  async function load() {
    try {
      [regimes, presets] = await Promise.all([
        call(commands.listTaxRegimes()),
        call(commands.taxPresets()),
      ]);
    } catch (err) {
      showError(err);
    }
  }

  $effect(() => {
    void load();
  });

  async function startNew() {
    const draft = await commands.taxPreset("custom", today);
    if (draft) editing = { id: null, draft };
  }

  async function usePreset(key: string) {
    if (!editing) return;
    const preset = await commands.taxPreset(key, editing.draft.validFrom);
    if (preset) editing.draft = preset;
  }

  async function save(event: SubmitEvent) {
    event.preventDefault();
    if (!editing) return;
    saving = true;
    try {
      await call(commands.saveTaxRegime(editing.id, $state.snapshot(editing.draft)));
      editing = null;
      await load();
    } catch (err) {
      showError(err);
    } finally {
      saving = false;
    }
  }

  async function remove(regime: TaxRegime) {
    const ok = await confirm({
      title: m.taxes_delete_title(),
      confirmLabel: m.ui_delete(),
      danger: true,
    });
    if (!ok) return;
    try {
      await call(commands.deleteTaxRegime(regime.id));
      await load();
    } catch (err) {
      showError(err);
    }
  }

  let presetOptions = $derived([
    ...presets.map((p) => ({ value: p.key, label: p.labels[i18n.locale] ?? p.labels.en ?? p.key })),
    { value: "custom", label: m.taxes_custom() },
  ]);
  const kindOptions = [
    { value: ANY, label: m.taxes_any_client() },
    { value: "person", label: m.taxes_people() },
    { value: "company", label: m.taxes_companies() },
  ];
  const currencyOptions = [
    { value: ANY, label: m.taxes_any_currency() },
    ...COMMON_CURRENCIES.map((c) => ({ value: c, label: c })),
  ];
</script>

<div class="flex max-w-2xl flex-col gap-4">
  <Switch
    label={m.taxes_enabled()}
    bind:checked={
      () => appSettings.value.taxesEnabled, (on) => void updateSettings({ taxesEnabled: on })
    }
  />
  <p class="text-sm text-muted">{m.taxes_estimate_note()}</p>

  <div class="flex items-center justify-between">
    <h3 class="text-sm font-semibold text-fg">{m.taxes_regimes()}</h3>
    <Button size="sm" onclick={startNew}>{m.taxes_add()}</Button>
  </div>
  {#if regimes && regimes.length === 0}
    <p class="text-sm text-muted">{m.taxes_none()}</p>
  {:else if regimes}
    <ul class="flex flex-col divide-y divide-border rounded-[var(--radius)] border border-border">
      {#each regimes as regime (regime.id)}
        <li class="flex items-center gap-3 px-3 py-2 text-sm">
          <span class="flex-1 text-fg">{regime.name || m.taxes_custom()}</span>
          <span class="text-muted"
            >{m.taxes_from({ date: formatIsoDate(regime.validFrom, i18n.locale) })}</span
          >
          <Button
            variant="ghost"
            size="sm"
            aria-label={m.taxes_edit({ name: regime.name })}
            onclick={() => {
              const { id, ...rest } = $state.snapshot(regime);
              const draft: Omit<typeof rest, "createdAt"> & { createdAt?: unknown } = rest;
              delete draft.createdAt;
              editing = { id, draft };
            }}
          >
            <Pencil class="size-4" aria-hidden="true" />
          </Button>
          <Button
            variant="ghost"
            size="sm"
            aria-label={m.taxes_delete({ name: regime.name })}
            onclick={() => remove(regime)}
          >
            <Trash2 class="size-4" aria-hidden="true" />
          </Button>
        </li>
      {/each}
    </ul>
  {/if}
</div>

{#if editing}
  <Dialog open title={m.taxes_regime()} onclose={() => (editing = null)}>
    <form id="tax-regime-form" class="flex w-[34rem] max-w-full flex-col gap-4" onsubmit={save}>
      <div class="grid gap-3 sm:grid-cols-2">
        <Select
          label={m.taxes_preset()}
          options={presetOptions}
          bind:value={() => editing!.draft.preset, (v) => v && void usePreset(v)}
        />
        <DatePicker
          label={m.taxes_valid_from()}
          bind:value={() => editing!.draft.validFrom, (v) => v && (editing!.draft.validFrom = v)}
        />
      </div>
      <TextField label={m.taxes_name()} bind:value={editing.draft.name} />
      {#each editing.draft.rules as rule, i (i)}
        <div
          role="group"
          aria-label={m.taxes_rule({ n: i + 1 })}
          class="grid items-end gap-2 sm:grid-cols-[1fr_1fr_1fr_auto]"
        >
          <NumberField
            label={m.taxes_rate()}
            min={0}
            max={100}
            step={0.01}
            bind:value={() => rule.rateBp / 100, (v) => (rule.rateBp = Math.round((v ?? 0) * 100))}
          />
          <Select
            label={m.taxes_clients()}
            options={kindOptions}
            bind:value={
              () => rule.clientKind ?? ANY,
              (v) => (rule.clientKind = (v || null) as ClientKind | null)
            }
          />
          <Select
            label={m.taxes_currency()}
            options={currencyOptions}
            bind:value={() => rule.currency ?? ANY, (v) => (rule.currency = v || null)}
          />
          <Button
            variant="ghost"
            size="sm"
            aria-label={m.taxes_remove_rule({ n: i + 1 })}
            disabled={editing.draft.rules.length === 1}
            onclick={() => editing!.draft.rules.splice(i, 1)}
          >
            <Trash2 class="size-4" aria-hidden="true" />
          </Button>
        </div>
      {/each}
      <div>
        <Button
          size="sm"
          onclick={() => editing!.draft.rules.push({ rateBp: 0, clientKind: null, currency: null })}
        >
          {m.taxes_add_rule()}
        </Button>
      </div>
    </form>
    {#snippet footer()}
      <Button onclick={() => (editing = null)}>{m.ui_cancel()}</Button>
      <Button type="submit" form="tax-regime-form" variant="primary" loading={saving}
        >{m.ui_save()}</Button
      >
    {/snippet}
  </Dialog>
{/if}
