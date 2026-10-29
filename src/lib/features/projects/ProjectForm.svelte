<script lang="ts">
  import {
    commands,
    type BillingType,
    type Client_Serialize as Client,
    type ProjectInput_Deserialize as ProjectInput,
    type ProjectStatus,
    type Project_Serialize as Project,
  } from "$lib/api/bindings";
  import { AppError, call } from "$lib/api/client";
  import { COMMON_CURRENCIES, currencyLabel } from "$lib/app/currencies";
  import { appSettings } from "$lib/app/settings.svelte";
  import { codeMessage, errorMessage } from "$lib/i18n/errors";
  import { i18n } from "$lib/i18n/locale.svelte";
  import MarkdownEditor from "$lib/markdown/MarkdownEditor.svelte";
  import { m } from "$lib/paraglide/messages";
  import Button from "$lib/ui/Button.svelte";
  import DatePicker from "$lib/ui/DatePicker.svelte";
  import MoneyField from "$lib/ui/MoneyField.svelte";
  import RadioGroup from "$lib/ui/RadioGroup.svelte";
  import Select from "$lib/ui/Select.svelte";
  import TextField from "$lib/ui/TextField.svelte";
  import { toast } from "$lib/ui/toast.svelte";
  import { PROJECT_STATUSES, billingLabel, projectStatusLabel } from "./labels";

  let {
    project,
    clientId,
    onsaved,
    oncancel,
  }: {
    /** Project to edit; a new one is created when omitted. */
    project?: Project;
    /** Client of a new project, e.g. when created from the client card. */
    clientId?: string;
    onsaved: (project: Project) => void;
    oncancel?: () => void;
  } = $props();

  const INHERIT = "";

  // The props only seed the form.
  // svelte-ignore state_referenced_locally
  const initial = project;
  let draft = $state<ProjectInput>({
    clientId: initial?.clientId ?? clientId ?? "",
    name: initial?.name ?? "",
    descriptionMd: initial?.descriptionMd ?? "",
    status: initial?.status ?? "active",
    billingType: initial?.billingType ?? "fixed",
    currency: initial?.currency ?? null,
    rateMinor: initial?.rateMinor ?? null,
    budgetMinor: initial?.budgetMinor ?? null,
    startDate: initial?.startDate ?? null,
    deadline: initial?.deadline ?? null,
  });
  let savedId = $state(initial?.id);
  let clients = $state<Client[]>([]);
  let errors = $state<Record<string, string>>({});
  let saving = $state(false);

  let client = $derived(clients.find((c) => c.id === draft.clientId));
  let shownCurrency = $derived(
    draft.currency ?? client?.defaultCurrency ?? appSettings.value.defaultCurrency,
  );
  let clientOptions = $derived(clients.map((c) => ({ value: c.id, label: c.displayName })));
  let statusOptions = $derived(
    PROJECT_STATUSES.map((s) => ({ value: s, label: projectStatusLabel(s) })),
  );
  let billingOptions = $derived(
    (["fixed", "hourly"] as const).map((b) => ({ value: b, label: billingLabel(b) })),
  );
  let currencyOptions = $derived([
    { value: INHERIT, label: m.project_currency_inherit() },
    ...COMMON_CURRENCIES.map((code) => ({ value: code, label: currencyLabel(code, i18n.locale) })),
  ]);

  $effect(() => {
    call(commands.listClients({ query: null, status: null, archived: false, tagIds: [] })).then(
      (list) => (clients = list),
      (err) => toast.show({ kind: "error", message: errorMessage(err) }),
    );
  });

  const error = (field: string) => (errors[field] ? codeMessage(errors[field]) : undefined);

  function validate(): Record<string, string> {
    const result: Record<string, string> = {};
    if (!draft.clientId) result.clientId = "validation.required";
    if (draft.name.trim() === "") result.name = "validation.required";
    if (draft.billingType === "fixed" && draft.budgetMinor === null) {
      result.budgetMinor = "validation.required";
    }
    if (draft.startDate && draft.deadline && draft.deadline < draft.startDate) {
      result.deadline = "validation.before_start";
    }
    return result;
  }

  async function save(event: SubmitEvent) {
    event.preventDefault();
    errors = validate();
    if (Object.keys(errors).length > 0) return;
    const input: ProjectInput = {
      ...draft,
      name: draft.name.trim(),
      rateMinor: draft.billingType === "hourly" ? draft.rateMinor : null,
      budgetMinor: draft.billingType === "fixed" ? draft.budgetMinor : null,
    };
    saving = true;
    try {
      const saved = savedId
        ? await call(commands.updateProject(savedId, input))
        : await call(commands.createProject(input));
      savedId = saved.id;
      onsaved(saved);
    } catch (err) {
      const field = err instanceof AppError ? err.params.field : undefined;
      if (err instanceof AppError && typeof field === "string") errors = { [field]: err.code };
      else toast.show({ kind: "error", message: errorMessage(err) });
    } finally {
      saving = false;
    }
  }
</script>

<form class="flex max-w-2xl flex-col gap-6" novalidate onsubmit={save}>
  <div class="grid gap-4 sm:grid-cols-2">
    <Select
      label={m.project_field_client()}
      options={clientOptions}
      required
      error={error("clientId")}
      bind:value={() => draft.clientId || null, (v) => (draft.clientId = v ?? "")}
    />
    <Select
      label={m.project_field_status()}
      options={statusOptions}
      bind:value={() => draft.status, (v) => (draft.status = v as ProjectStatus)}
    />
  </div>
  <TextField
    label={m.project_field_name()}
    required
    bind:value={draft.name}
    error={error("name")}
  />

  <section class="flex flex-col gap-4">
    <RadioGroup
      label={m.project_field_billing()}
      options={billingOptions}
      bind:value={() => draft.billingType, (v) => (draft.billingType = v as BillingType)}
    />
    <div class="grid gap-4 sm:grid-cols-2">
      <Select
        label={m.project_field_currency()}
        options={currencyOptions}
        bind:value={
          () => draft.currency ?? INHERIT, (v) => (draft.currency = v === INHERIT ? null : v)
        }
      />
      {#if draft.billingType === "fixed"}
        <MoneyField
          label={m.project_field_budget()}
          currency={shownCurrency}
          required
          bind:value={draft.budgetMinor}
          error={error("budgetMinor")}
        />
      {:else}
        <MoneyField
          label={m.project_field_rate()}
          hint={m.project_field_rate_hint()}
          currency={shownCurrency}
          bind:value={draft.rateMinor}
          error={error("rateMinor")}
        />
      {/if}
    </div>
  </section>

  <div class="grid gap-4 sm:grid-cols-2">
    <DatePicker
      label={m.project_field_start()}
      bind:value={draft.startDate}
      error={error("startDate")}
    />
    <DatePicker
      label={m.project_field_deadline()}
      bind:value={draft.deadline}
      error={error("deadline")}
    />
  </div>

  <MarkdownEditor label={m.project_field_description()} bind:value={draft.descriptionMd} />

  <div class="flex gap-2">
    <Button type="submit" variant="primary" loading={saving}>{m.ui_save()}</Button>
    {#if oncancel}
      <Button onclick={oncancel}>{m.ui_cancel()}</Button>
    {/if}
  </div>
</form>
