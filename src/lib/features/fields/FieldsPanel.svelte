<script lang="ts">
  import { untrack } from "svelte";
  import ArrowDown from "@lucide/svelte/icons/arrow-down";
  import ArrowUp from "@lucide/svelte/icons/arrow-up";
  import Pencil from "@lucide/svelte/icons/pencil";
  import Plus from "@lucide/svelte/icons/plus";
  import Trash2 from "@lucide/svelte/icons/trash-2";
  import {
    commands,
    type CustomField,
    type CustomFieldInput,
    type EntityRef,
    type FieldType,
  } from "$lib/api/bindings";
  import { AppError, call } from "$lib/api/client";
  import { codeMessage, errorMessage } from "$lib/i18n/errors";
  import { formatIsoDate } from "$lib/i18n/format";
  import { i18n } from "$lib/i18n/locale.svelte";
  import { m } from "$lib/paraglide/messages";
  import Button from "$lib/ui/Button.svelte";
  import DatePicker from "$lib/ui/DatePicker.svelte";
  import Dialog from "$lib/ui/Dialog.svelte";
  import { confirm } from "$lib/ui/dialogs.svelte";
  import Select from "$lib/ui/Select.svelte";
  import Spinner from "$lib/ui/Spinner.svelte";
  import TextField from "$lib/ui/TextField.svelte";
  import { toast } from "$lib/ui/toast.svelte";

  let { owner }: { owner: EntityRef } = $props();

  const TYPES: FieldType[] = ["text", "url", "email", "phone", "date", "number"];
  const formId = $props.id();

  let fields = $state<CustomField[] | null>(null);
  /** Field being edited in the dialog; `id` is null for a new one. */
  let editing = $state<(CustomFieldInput & { id: string | null }) | null>(null);
  let errors = $state<Record<string, string>>({});
  let busy = $state(false);

  let typeOptions = $derived(TYPES.map((t) => ({ value: t, label: typeLabel(t) })));

  function typeLabel(type: FieldType): string {
    switch (type) {
      case "text":
        return m.field_type_text();
      case "url":
        return m.field_type_url();
      case "email":
        return m.field_type_email();
      case "phone":
        return m.field_type_phone();
      case "date":
        return m.field_type_date();
      case "number":
        return m.field_type_number();
    }
  }

  function display(field: CustomField): string {
    if (field.fieldType === "date" && field.value) {
      return formatIsoDate(field.value, i18n.locale);
    }
    return field.value;
  }

  async function run(action: () => Promise<unknown>): Promise<boolean> {
    busy = true;
    try {
      await action();
      fields = await call(commands.listCustomFields(owner));
      return true;
    } catch (err) {
      toast.show({ kind: "error", message: errorMessage(err) });
      return false;
    } finally {
      busy = false;
    }
  }

  // Reload when the owner changes (it is read after an await in `run`).
  $effect(() => {
    void [owner.type, owner.id];
    untrack(() => run(async () => {}));
  });

  function open(field?: CustomField) {
    errors = {};
    editing = field
      ? { id: field.id, key: field.key, value: field.value, fieldType: field.fieldType }
      : { id: null, key: "", value: "", fieldType: "text" };
  }

  async function save(event: SubmitEvent) {
    event.preventDefault();
    if (!editing) return;
    const { id, ...input } = editing;
    if (input.key.trim() === "") {
      errors = { key: "validation.required" };
      return;
    }
    busy = true;
    try {
      if (id) await call(commands.updateCustomField(id, input));
      else await call(commands.createCustomField(owner, input));
      editing = null;
      fields = await call(commands.listCustomFields(owner));
    } catch (err) {
      const field = err instanceof AppError ? err.params.field : undefined;
      if (err instanceof AppError && typeof field === "string") errors = { [field]: err.code };
      else toast.show({ kind: "error", message: errorMessage(err) });
    } finally {
      busy = false;
    }
  }

  function move(index: number, delta: number) {
    if (!fields) return;
    const ids = fields.map((f) => f.id);
    [ids[index], ids[index + delta]] = [ids[index + delta], ids[index]];
    void run(() => call(commands.reorderCustomFields(owner, ids)));
  }

  async function remove(field: CustomField) {
    const ok = await confirm({
      title: m.field_delete_title({ name: field.key }),
      confirmLabel: m.ui_delete(),
      danger: true,
    });
    if (ok) void run(() => call(commands.deleteCustomField(field.id)));
  }

  const error = (name: string) => (errors[name] ? codeMessage(errors[name]) : undefined);
</script>

<div class="flex flex-col gap-4">
  {#if fields === null}
    <div class="flex justify-center p-6"><Spinner /></div>
  {:else if fields.length === 0}
    <p class="text-sm text-muted">{m.fields_empty()}</p>
  {:else}
    <ul class="flex flex-col divide-y divide-border rounded-[var(--radius)] border border-border">
      {#each fields as field, i (field.id)}
        <li class="flex items-center gap-3 px-3 py-2 text-sm">
          <span class="w-48 shrink-0 truncate text-muted" data-testid="field-key">{field.key}</span>
          <span class="min-w-0 flex-1 break-words text-fg">{display(field)}</span>
          <span class="flex shrink-0 gap-1">
            <Button
              variant="ghost"
              size="sm"
              aria-label={m.field_move_up()}
              disabled={busy || i === 0}
              onclick={() => move(i, -1)}
            >
              <ArrowUp class="size-4" aria-hidden="true" />
            </Button>
            <Button
              variant="ghost"
              size="sm"
              aria-label={m.field_move_down()}
              disabled={busy || i === fields.length - 1}
              onclick={() => move(i, 1)}
            >
              <ArrowDown class="size-4" aria-hidden="true" />
            </Button>
            <Button variant="ghost" size="sm" aria-label={m.ui_edit()} onclick={() => open(field)}>
              <Pencil class="size-4" aria-hidden="true" />
            </Button>
            <Button
              variant="ghost"
              size="sm"
              aria-label={m.ui_delete()}
              onclick={() => remove(field)}
            >
              <Trash2 class="size-4" aria-hidden="true" />
            </Button>
          </span>
        </li>
      {/each}
    </ul>
  {/if}
  <div>
    <Button onclick={() => open()}>
      <Plus class="size-4" aria-hidden="true" />
      {m.fields_add()}
    </Button>
  </div>
</div>

{#if editing}
  <Dialog open title={editing.id ? m.field_edit() : m.field_new()} onclose={() => (editing = null)}>
    <form id={formId} class="flex w-96 max-w-full flex-col gap-4" novalidate onsubmit={save}>
      <TextField label={m.field_key()} bind:value={editing.key} error={error("key")} />
      <Select
        label={m.field_type()}
        options={typeOptions}
        bind:value={() => editing!.fieldType, (v) => (editing!.fieldType = v as FieldType)}
      />
      {#if editing.fieldType === "date"}
        <DatePicker
          label={m.field_value()}
          bind:value={() => editing!.value || null, (v) => (editing!.value = v ?? "")}
          error={error("value")}
        />
      {:else}
        <TextField label={m.field_value()} bind:value={editing.value} error={error("value")} />
      {/if}
    </form>
    {#snippet footer()}
      <Button onclick={() => (editing = null)}>{m.ui_cancel()}</Button>
      <Button type="submit" form={formId} variant="primary" loading={busy}>{m.ui_save()}</Button>
    {/snippet}
  </Dialog>
{/if}
