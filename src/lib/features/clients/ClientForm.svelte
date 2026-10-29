<script lang="ts">
  import Plus from "@lucide/svelte/icons/plus";
  import X from "@lucide/svelte/icons/x";
  import {
    commands,
    type Client_Serialize as Client,
    type ClientKind,
    type ClientStatus,
    type ContactMethodKind,
    type ContactMethod_Serialize as ContactMethod,
  } from "$lib/api/bindings";
  import { AppError, call } from "$lib/api/client";
  import { COMMON_CURRENCIES, currencyLabel } from "$lib/app/currencies";
  import { appSettings } from "$lib/app/settings.svelte";
  import { codeMessage, errorMessage } from "$lib/i18n/errors";
  import { i18n } from "$lib/i18n/locale.svelte";
  import { m } from "$lib/paraglide/messages";
  import Button from "$lib/ui/Button.svelte";
  import MoneyField from "$lib/ui/MoneyField.svelte";
  import RadioGroup from "$lib/ui/RadioGroup.svelte";
  import Select from "$lib/ui/Select.svelte";
  import TextField from "$lib/ui/TextField.svelte";
  import { toast } from "$lib/ui/toast.svelte";
  import {
    emptyDraft,
    fromClient,
    toInput,
    toMethods,
    validateClient,
    type ClientDraft,
    type FormErrors,
  } from "./form";
  import { CLIENT_STATUSES, statusLabel } from "./status";

  let {
    client,
    methods = [],
    onsaved,
    oncancel,
  }: {
    /** Client to edit; a new one is created when omitted. */
    client?: Client;
    methods?: ContactMethod[];
    onsaved: (client: Client) => void;
    oncancel?: () => void;
  } = $props();

  const METHOD_KINDS: ContactMethodKind[] = [
    "email",
    "phone",
    "telegram",
    "whatsapp",
    "website",
    "other",
  ];
  const DEFAULT_CURRENCY = "";

  // The props only seed the form; later prop changes do not reset edits.
  // svelte-ignore state_referenced_locally
  let draft = $state<ClientDraft>(client ? fromClient(client, methods) : emptyDraft());
  // svelte-ignore state_referenced_locally
  let savedId = $state(client?.id);
  let errors = $state<FormErrors>({});
  let saving = $state(false);

  let defaultCurrency = $derived(appSettings.value.defaultCurrency);
  let currencyOptions = $derived([
    { value: DEFAULT_CURRENCY, label: m.client_currency_default({ code: defaultCurrency }) },
    ...COMMON_CURRENCIES.map((code) => ({ value: code, label: currencyLabel(code, i18n.locale) })),
  ]);
  let kindOptions = $derived([
    { value: "company", label: m.client_kind_company() },
    { value: "person", label: m.client_kind_person() },
  ]);
  let statusOptions = $derived(CLIENT_STATUSES.map((s) => ({ value: s, label: statusLabel(s) })));
  let methodOptions = $derived(METHOD_KINDS.map((k) => ({ value: k, label: methodLabel(k) })));

  function methodLabel(kind: ContactMethodKind): string {
    switch (kind) {
      case "email":
        return m.contact_method_email();
      case "phone":
        return m.contact_method_phone();
      case "telegram":
        return m.contact_method_telegram();
      case "whatsapp":
        return m.contact_method_whatsapp();
      case "website":
        return m.contact_method_website();
      case "other":
        return m.contact_method_other();
    }
  }

  const error = (path: string) => (errors[path] ? codeMessage(errors[path]) : undefined);

  function addMethod() {
    draft.methods.push({ kind: "email", value: "", label: "" });
  }

  function removeMethod(index: number) {
    draft.methods.splice(index, 1);
    errors = {};
  }

  /** Puts a backend validation error under its field, if it has one. */
  function fieldOf(err: unknown, methodsStep: boolean): string | undefined {
    if (!(err instanceof AppError) || !err.code.startsWith("validation.")) return undefined;
    const { field, index } = err.params;
    if (methodsStep && typeof index === "number") return `methods.${index}.value`;
    return typeof field === "string" ? field : undefined;
  }

  async function save(event: SubmitEvent) {
    event.preventDefault();
    errors = validateClient(draft);
    if (Object.keys(errors).length > 0) return;

    saving = true;
    let methodsStep = false;
    try {
      const input = toInput(draft);
      // After a partial failure the client already exists, so update it.
      const result = savedId
        ? await call(commands.updateClient(savedId, input))
        : await call(commands.createClient(input));
      savedId = result.id;
      methodsStep = true;
      await call(
        commands.replaceContactMethods({ type: "client", id: result.id }, toMethods(draft)),
      );
      onsaved(result);
    } catch (err) {
      const path = fieldOf(err, methodsStep);
      if (path && err instanceof AppError) errors = { [path]: err.code };
      else toast.show({ kind: "error", message: errorMessage(err) });
    } finally {
      saving = false;
    }
  }
</script>

<form class="flex max-w-2xl flex-col gap-8" novalidate onsubmit={save}>
  <section class="flex flex-col gap-4">
    <RadioGroup
      label={m.client_field_kind()}
      options={kindOptions}
      bind:value={() => draft.kind, (v) => (draft.kind = v as ClientKind)}
    />
    {#if draft.kind === "person"}
      <div class="grid gap-4 sm:grid-cols-3">
        <TextField
          label={m.client_field_last_name()}
          bind:value={draft.lastName}
          error={error("lastName")}
        />
        <TextField
          label={m.client_field_first_name()}
          bind:value={draft.firstName}
          error={error("firstName")}
        />
        <TextField label={m.client_field_middle_name()} bind:value={draft.middleName} />
      </div>
    {:else}
      <TextField
        label={m.client_field_company_name()}
        bind:value={draft.companyName}
        error={error("companyName")}
      />
      <TextField
        label={m.client_field_legal_name()}
        hint={m.client_field_legal_name_hint()}
        bind:value={draft.legalName}
      />
    {/if}
    <div class="w-56">
      <Select
        label={m.client_field_status()}
        options={statusOptions}
        bind:value={() => draft.status, (v) => (draft.status = v as ClientStatus)}
      />
    </div>
  </section>

  <fieldset class="flex flex-col gap-3">
    <legend class="mb-3 text-base font-semibold text-fg">{m.contact_methods()}</legend>
    {#each draft.methods as method, i (method)}
      <div class="grid grid-cols-[10rem_1fr_10rem_auto] items-start gap-2">
        <Select
          label={m.contact_method_kind()}
          options={methodOptions}
          bind:value={() => method.kind, (v) => (method.kind = v as ContactMethodKind)}
        />
        <TextField
          label={m.contact_method_value()}
          bind:value={method.value}
          error={error(`methods.${i}.value`)}
        />
        <TextField label={m.contact_method_label()} bind:value={method.label} />
        <div class="pt-6">
          <Button
            variant="ghost"
            aria-label={m.contact_method_remove({ index: i + 1 })}
            onclick={() => removeMethod(i)}
          >
            <X class="size-4" aria-hidden="true" />
          </Button>
        </div>
      </div>
    {/each}
    <div>
      <Button variant="ghost" onclick={addMethod}>
        <Plus class="size-4" aria-hidden="true" />
        {m.contact_method_add()}
      </Button>
    </div>
  </fieldset>

  <section class="flex flex-col gap-4">
    <h2 class="text-base font-semibold text-fg">{m.client_section_billing()}</h2>
    <div class="grid gap-4 sm:grid-cols-2">
      <Select
        label={m.client_field_currency()}
        options={currencyOptions}
        bind:value={
          () => draft.currency ?? DEFAULT_CURRENCY,
          (v) => (draft.currency = v === DEFAULT_CURRENCY ? null : v)
        }
      />
      <MoneyField
        label={m.client_field_rate()}
        currency={draft.currency ?? defaultCurrency}
        bind:value={draft.rateMinor}
        error={error("rateMinor")}
      />
    </div>
  </section>

  <div class="flex gap-2">
    <Button type="submit" variant="primary" loading={saving}>{m.ui_save()}</Button>
    {#if oncancel}
      <Button onclick={oncancel}>{m.ui_cancel()}</Button>
    {/if}
  </div>
</form>
