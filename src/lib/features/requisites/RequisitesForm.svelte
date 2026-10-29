<script lang="ts">
  import { untrack } from "svelte";
  import {
    commands,
    type CountrySchema_Serialize as CountrySchema,
    type FieldDef_Serialize as FieldDef,
    type RequisitesOwner,
  } from "$lib/api/bindings";
  import { call } from "$lib/api/client";
  import { codeMessage, errorMessage } from "$lib/i18n/errors";
  import { m } from "$lib/paraglide/messages";
  import Button from "$lib/ui/Button.svelte";
  import { confirm } from "$lib/ui/dialogs.svelte";
  import Select from "$lib/ui/Select.svelte";
  import Spinner from "$lib/ui/Spinner.svelte";
  import TextArea from "$lib/ui/TextArea.svelte";
  import TextField from "$lib/ui/TextField.svelte";
  import { toast } from "$lib/ui/toast.svelte";
  import { label, loadSchemas, mask } from "./schemas";

  let {
    owner,
    withName = false,
  }: {
    owner: RequisitesOwner;
    /** Show the "name on documents" field (the profile has no other name). */
    withName?: boolean;
  } = $props();

  let schemas = $state<CountrySchema[] | null>(null);
  let country = $state("");
  let legalForm = $state("");
  let name = $state("");
  let values = $state<Record<string, string>>({});
  /** Country of the saved requisites; changing it must be confirmed. */
  let savedCountry = $state<string | null>(null);
  /** Field key → error rule. */
  let errors = $state<Record<string, string>>({});
  let saving = $state(false);

  let schema = $derived(schemas?.find((s) => s.code === country));
  let form = $derived(schema?.legalForms.find((f) => f.key === legalForm));
  let countryOptions = $derived(
    (schemas ?? []).map((s) => ({ value: s.code, label: label(s.labels) })),
  );
  let formOptions = $derived(
    (schema?.legalForms ?? []).map((f) => ({ value: f.key, label: label(f.labels) })),
  );
  /** The form's fields grouped by the schema's groups, in schema order. */
  let groups = $derived.by(() => {
    if (!schema || !form) return [];
    const s = schema;
    return s.groups
      .map((group) => ({
        group,
        fields: form.fields.flatMap((ff) => {
          const def = s.fields.find((d) => d.key === ff.key);
          return def && def.group === group.key ? [{ def, required: ff.required }] : [];
        }),
      }))
      .filter((g) => g.fields.length > 0);
  });

  $effect(() => {
    const target = owner;
    untrack(() => {
      Promise.all([loadSchemas(), call(commands.getRequisites(target))]).then(
        ([all, saved]) => {
          schemas = all;
          if (saved) {
            country = saved.country;
            legalForm = saved.legalForm;
            name = saved.name ?? "";
            values = { ...saved.values };
            savedCountry = saved.country;
          } else {
            country = all[0]?.code ?? "";
            legalForm = all[0]?.legalForms[0]?.key ?? "";
          }
        },
        (err) => toast.show({ kind: "error", message: errorMessage(err) }),
      );
    });
  });

  async function changeCountry(code: string | null) {
    const next = schemas?.find((s) => s.code === code);
    if (!next || next.code === country) return;
    const keep = new Set(next.fields.map((f) => f.key));
    const lost = Object.entries(values).some(([key, v]) => !keep.has(key) && v.trim() !== "");
    if (
      lost &&
      !(await confirm({
        title: m.requisites_change_country_title(),
        message: m.requisites_change_country_message({ country: label(next.labels) }),
        confirmLabel: m.requisites_change_country_confirm(),
        danger: true,
      }))
    ) {
      return;
    }
    country = next.code;
    legalForm = next.legalForms.some((f) => f.key === legalForm)
      ? legalForm
      : (next.legalForms[0]?.key ?? "");
    values = Object.fromEntries(Object.entries(values).filter(([key]) => keep.has(key)));
    errors = {};
  }

  function onInput(def: FieldDef, event: Event) {
    const input = event.currentTarget as HTMLInputElement | HTMLTextAreaElement;
    const masked = mask(def, input.value);
    if (masked !== input.value) input.value = masked;
    values[def.key] = masked;
    delete errors[def.key];
  }

  function error(def: FieldDef): string | undefined {
    const rule = errors[def.key];
    return rule ? codeMessage(`validation.${rule}`, { max: def.maxLength }) : undefined;
  }

  async function save(event: SubmitEvent) {
    event.preventDefault();
    saving = true;
    try {
      const result = await call(
        commands.saveRequisites(owner, {
          country,
          legalForm,
          name: withName && name.trim() !== "" ? name.trim() : null,
          values: Object.fromEntries(Object.entries(values).filter(([, v]) => v.trim() !== "")),
          confirmCountryChange: savedCountry !== null && savedCountry !== country,
        }),
      );
      if (result.status === "invalid") {
        errors = Object.fromEntries(result.errors.map((e) => [e.field, e.rule]));
        return;
      }
      errors = {};
      savedCountry = result.requisites.country;
      values = { ...result.requisites.values };
      toast.show({ message: m.requisites_saved() });
    } catch (err) {
      toast.show({ kind: "error", message: errorMessage(err) });
    } finally {
      saving = false;
    }
  }
</script>

{#if !schemas}
  <div class="flex justify-center p-6"><Spinner /></div>
{:else}
  <form class="flex max-w-2xl flex-col gap-6" novalidate onsubmit={save}>
    <div class="grid gap-4 sm:grid-cols-2">
      <Select
        label={m.requisites_country()}
        options={countryOptions}
        bind:value={() => country, (code) => void changeCountry(code)}
      />
      <Select
        label={m.requisites_legal_form()}
        options={formOptions}
        bind:value={
          () => legalForm,
          (key) => {
            legalForm = key ?? "";
            errors = {};
          }
        }
      />
    </div>
    {#if withName}
      <TextField label={m.requisites_name()} hint={m.requisites_name_hint()} bind:value={name} />
    {/if}
    {#each groups as { group, fields } (group.key)}
      <fieldset class="flex flex-col gap-4">
        <legend class="mb-2 text-base font-semibold text-fg">{label(group.labels)}</legend>
        {#each fields as { def, required } (def.key)}
          {#if def.kind === "textarea"}
            <TextArea
              label={label(def.labels)}
              rows={2}
              {required}
              value={values[def.key] ?? ""}
              error={error(def)}
              oninput={(e: Event) => onInput(def, e)}
            />
          {:else}
            <TextField
              label={label(def.labels)}
              {required}
              value={values[def.key] ?? ""}
              error={error(def)}
              inputmode={def.input === "digits" ? "numeric" : undefined}
              autocomplete="off"
              oninput={(e: Event) => onInput(def, e)}
            />
          {/if}
        {/each}
      </fieldset>
    {/each}
    <div>
      <Button type="submit" variant="primary" loading={saving}>{m.ui_save()}</Button>
    </div>
  </form>
{/if}
