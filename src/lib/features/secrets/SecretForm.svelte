<script lang="ts">
  import KeyRound from "@lucide/svelte/icons/key-round";
  import Plus from "@lucide/svelte/icons/plus";
  import Wand from "@lucide/svelte/icons/wand-sparkles";
  import X from "@lucide/svelte/icons/x";
  import {
    commands,
    type Client_Serialize as Client,
    type CustomEntry,
    type Project_Serialize as Project,
    type SecretInput_Deserialize as SecretInput,
    type SecretKind,
    type SecretPayload,
    type SecretTemplate,
    type Secret_Serialize as Secret,
  } from "$lib/api/bindings";
  import { AppError, call } from "$lib/api/client";
  import { codeMessage, errorMessage } from "$lib/i18n/errors";
  import { m } from "$lib/paraglide/messages";
  import Button from "$lib/ui/Button.svelte";
  import Checkbox from "$lib/ui/Checkbox.svelte";
  import DatePicker from "$lib/ui/DatePicker.svelte";
  import Select from "$lib/ui/Select.svelte";
  import TextArea from "$lib/ui/TextArea.svelte";
  import TextField from "$lib/ui/TextField.svelte";
  import { toast } from "$lib/ui/toast.svelte";
  import { fieldLabel, kindLabel } from "./labels";
  import PasswordGenerator from "./PasswordGenerator.svelte";
  import SshKeyDialog from "./SshKeyDialog.svelte";

  let {
    secret,
    payload,
    clientId,
    projectId,
    onsaved,
    oncancel,
  }: {
    /** Secret to edit (with its decrypted `payload`); new when omitted. */
    secret?: Secret;
    payload?: SecretPayload;
    clientId?: string;
    projectId?: string;
    onsaved: (secret: Secret) => void;
    oncancel?: () => void;
  } = $props();

  const NONE = "";

  // The props only seed the form.
  // svelte-ignore state_referenced_locally
  const seed = { secret, payload, clientId, projectId };
  let title = $state(seed.secret?.title ?? "");
  let kind = $state<SecretKind>(seed.secret?.kind ?? "login");
  let client = $state<string | null>(seed.secret?.clientId ?? seed.clientId ?? null);
  let project = $state<string | null>(seed.secret?.projectId ?? seed.projectId ?? null);
  let expiresOn = $state<string | null>(seed.secret?.expiresOn ?? null);
  /** Values by template key; kept across type changes, filtered on save. */
  let values = $state<Record<string, string>>({ ...(seed.payload?.fields ?? {}) });
  let custom = $state<CustomEntry[]>(seed.payload?.custom.map((c) => ({ ...c })) ?? []);
  let savedId = $state(seed.secret?.id);

  let templates = $state<SecretTemplate[]>([]);
  let clients = $state<Client[]>([]);
  let projects = $state<Project[]>([]);
  let errors = $state<Record<string, string>>({});
  let saving = $state(false);
  /** Template key the password generator fills. */
  let generatingFor = $state<string | null>(null);
  let generatingSsh = $state(false);

  let template = $derived(templates.find((t) => t.kind === kind));
  let kindOptions = $derived(templates.map((t) => ({ value: t.kind, label: kindLabel(t.kind) })));
  let linkedProject = $derived(projects.find((p) => p.id === project));
  let clientOptions = $derived(
    linkedProject
      ? [{ value: linkedProject.clientId, label: linkedProject.clientName }]
      : [
          { value: NONE, label: m.task_no_client() },
          ...clients.map((c) => ({ value: c.id, label: c.displayName })),
        ],
  );
  let projectOptions = $derived([
    { value: NONE, label: m.task_no_project() },
    ...projects
      .filter((p) => !client || p.clientId === client)
      .map((p) => ({ value: p.id, label: p.name })),
  ]);

  const showError = (err: unknown) => toast.show({ kind: "error", message: errorMessage(err) });

  $effect(() => {
    Promise.all([
      commands.listSecretTemplates(),
      call(commands.listClients({ query: null, status: null, archived: false, tagIds: [] })),
      call(commands.listProjects({ query: null, status: null, clientId: null, archived: false })),
    ]).then(([t, c, p]) => {
      templates = t;
      clients = c;
      projects = p;
      const linked = p.find((x) => x.id === project);
      if (linked) client = linked.clientId;
    }, showError);
  });

  function setProject(id: string | null) {
    project = id || null;
    const linked = projects.find((p) => p.id === project);
    if (linked) client = linked.clientId;
  }

  function setClient(id: string | null) {
    client = id || null;
    if (linkedProject && linkedProject.clientId !== client) project = null;
  }

  const error = (key: string) => (errors[key] ? codeMessage(errors[key]) : undefined);

  function validate(): Record<string, string> {
    const result: Record<string, string> = {};
    if (!title.trim()) result.title = "validation.required";
    for (const f of template?.fields ?? []) {
      if (f.required && !(values[f.key] ?? "").trim()) {
        result[`fields.${f.key}`] = "validation.required";
      }
    }
    return result;
  }

  async function save(event: SubmitEvent) {
    event.preventDefault();
    errors = validate();
    if (Object.keys(errors).length > 0) return;
    const fields = Object.fromEntries(
      (template?.fields ?? [])
        .map((f) => [f.key, values[f.key] ?? ""] as const)
        .filter(([, value]) => value.trim() !== ""),
    );
    const input: SecretInput = {
      title: title.trim(),
      kind,
      clientId: client,
      projectId: project,
      expiresOn,
      payload: {
        fields,
        custom: custom.filter((c) => c.key.trim() || c.value.trim()),
      },
    };
    saving = true;
    try {
      const saved = savedId
        ? await call(commands.updateSecret(savedId, input))
        : await call(commands.createSecret(input));
      savedId = saved.id;
      onsaved(saved);
    } catch (err) {
      const field = err instanceof AppError ? err.params.field : undefined;
      if (err instanceof AppError && typeof field === "string") errors = { [field]: err.code };
      else showError(err);
    } finally {
      saving = false;
    }
  }
</script>

<form class="flex max-w-2xl flex-col gap-6" novalidate onsubmit={save}>
  <div class="grid gap-4 sm:grid-cols-2">
    <TextField label={m.secret_field_title()} required bind:value={title} error={error("title")} />
    <Select
      label={m.secret_field_kind()}
      options={kindOptions}
      bind:value={() => kind, (v) => (kind = v as SecretKind)}
    />
    <Select
      label={m.task_field_client()}
      options={clientOptions}
      bind:value={() => client ?? NONE, setClient}
    />
    <Select
      label={m.task_field_project()}
      options={projectOptions}
      bind:value={() => project ?? NONE, setProject}
    />
    <DatePicker
      label={m.secret_field_expires()}
      bind:value={expiresOn}
      error={error("expiresOn")}
    />
  </div>

  <fieldset class="flex flex-col gap-4">
    <legend class="mb-2 text-base font-semibold text-fg">{m.secret_values()}</legend>
    {#if kind === "ssh_key"}
      <div>
        <Button onclick={() => (generatingSsh = true)}>
          <KeyRound class="size-4" aria-hidden="true" />
          {m.ssh_generate()}
        </Button>
      </div>
    {/if}
    {#each template?.fields ?? [] as f (f.key)}
      {#if f.multiline}
        <TextArea
          label={fieldLabel(f.key)}
          rows={f.secret ? 6 : 3}
          required={f.required}
          spellcheck="false"
          class="font-mono"
          bind:value={() => values[f.key] ?? "", (v) => (values[f.key] = v)}
          error={error(`fields.${f.key}`)}
        />
      {:else}
        <div class="flex items-start gap-2">
          <div class="min-w-0 flex-1">
            <TextField
              label={fieldLabel(f.key)}
              type={f.secret ? "password" : "text"}
              autocomplete={f.secret ? "new-password" : "off"}
              required={f.required}
              bind:value={() => values[f.key] ?? "", (v) => (values[f.key] = v)}
              error={error(`fields.${f.key}`)}
            />
          </div>
          {#if f.secret}
            <div class="pt-6">
              <Button
                variant="ghost"
                aria-label={m.generator_open({ name: fieldLabel(f.key) })}
                onclick={() => (generatingFor = f.key)}
              >
                <Wand class="size-4" aria-hidden="true" />
              </Button>
            </div>
          {/if}
        </div>
      {/if}
    {/each}
  </fieldset>

  <fieldset class="flex flex-col gap-3">
    <legend class="mb-2 text-base font-semibold text-fg">{m.secret_custom()}</legend>
    {#each custom as entry, i (entry)}
      <div class="grid grid-cols-[12rem_1fr_auto_auto] items-start gap-2">
        <TextField
          label={m.secret_custom_key()}
          bind:value={entry.key}
          error={error(`custom.${i}.key`)}
        />
        <TextField
          label={m.secret_custom_value()}
          type={entry.secret ? "password" : "text"}
          autocomplete="off"
          bind:value={entry.value}
        />
        <div class="pt-7">
          <Checkbox label={m.secret_custom_secret()} bind:checked={entry.secret} />
        </div>
        <div class="pt-6">
          <Button
            variant="ghost"
            aria-label={m.secret_custom_remove({ index: i + 1 })}
            onclick={() => custom.splice(i, 1)}
          >
            <X class="size-4" aria-hidden="true" />
          </Button>
        </div>
      </div>
    {/each}
    <div>
      <Button variant="ghost" onclick={() => custom.push({ key: "", value: "", secret: true })}>
        <Plus class="size-4" aria-hidden="true" />
        {m.secret_custom_add()}
      </Button>
    </div>
  </fieldset>

  <div class="flex gap-2">
    <Button type="submit" variant="primary" loading={saving}>{m.ui_save()}</Button>
    {#if oncancel}
      <Button onclick={oncancel}>{m.ui_cancel()}</Button>
    {/if}
  </div>
</form>

{#if generatingFor}
  {@const key = generatingFor}
  <PasswordGenerator
    onuse={(value) => {
      values[key] = value;
      generatingFor = null;
    }}
    oncancel={() => (generatingFor = null)}
  />
{/if}

{#if generatingSsh}
  <SshKeyDialog
    ongenerated={(key, passphrase) => {
      values.private_key = key.privateKey;
      values.public_key = key.publicKey;
      if (passphrase) values.passphrase = passphrase;
      generatingSsh = false;
    }}
    oncancel={() => (generatingSsh = false)}
  />
{/if}
