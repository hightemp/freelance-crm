<script lang="ts">
  import type {
    StatusCategory,
    StatusInput,
    TaskStatus_Serialize as TaskStatus,
  } from "$lib/api/bindings";
  import { codeMessage } from "$lib/i18n/errors";
  import { m } from "$lib/paraglide/messages";
  import Button from "$lib/ui/Button.svelte";
  import ColorPicker from "$lib/ui/ColorPicker.svelte";
  import Dialog from "$lib/ui/Dialog.svelte";
  import NumberField from "$lib/ui/NumberField.svelte";
  import Select from "$lib/ui/Select.svelte";
  import TextField from "$lib/ui/TextField.svelte";
  import { statusName } from "./statuses";

  let {
    column,
    saving = false,
    onsave,
    oncancel,
  }: {
    /** Column to edit; a new one is created when omitted. */
    column?: TaskStatus;
    saving?: boolean;
    onsave: (input: StatusInput) => void;
    oncancel: () => void;
  } = $props();

  const formId = $props.id();
  // svelte-ignore state_referenced_locally
  const initial = column;
  // A default column shows its localized name; leaving it unchanged keeps
  // it localized (name stays null).
  const shownName = initial ? statusName(initial) : "";
  let name = $state(shownName);
  let category = $state<StatusCategory>(initial?.category ?? "in_progress");
  let color = $state(initial?.color ?? "#64748b");
  let wipLimit = $state<number | null>(initial?.wipLimit ?? null);
  let error = $state<string | null>(null);

  let categories = $derived([
    { value: "todo", label: m.category_todo() },
    { value: "in_progress", label: m.category_in_progress() },
    { value: "done", label: m.category_done() },
  ]);

  function submit(event: SubmitEvent) {
    event.preventDefault();
    const trimmed = name.trim();
    const keepDefault = initial?.name === null && initial.nameKey !== null && trimmed === shownName;
    if (!trimmed && !keepDefault) {
      error = "validation.required";
      return;
    }
    onsave({
      name: keepDefault ? null : trimmed,
      category,
      color,
      wipLimit: wipLimit && wipLimit > 0 ? Math.round(wipLimit) : null,
    });
  }
</script>

<Dialog open title={initial ? m.column_edit() : m.column_new()} onclose={oncancel}>
  <form id={formId} class="flex w-80 max-w-full flex-col gap-4" novalidate onsubmit={submit}>
    <TextField
      label={m.column_name()}
      required
      bind:value={name}
      error={error ? codeMessage(error) : undefined}
    />
    <div class="flex flex-col gap-1">
      <Select
        label={m.column_category()}
        options={categories}
        bind:value={() => category, (v) => (category = v as StatusCategory)}
      />
      <p class="text-xs text-muted">{m.column_category_hint()}</p>
    </div>
    <ColorPicker label={m.column_color()} bind:value={color} />
    <NumberField label={m.column_wip()} hint={m.column_wip_hint()} min={1} bind:value={wipLimit} />
  </form>
  {#snippet footer()}
    <Button onclick={oncancel}>{m.ui_cancel()}</Button>
    <Button type="submit" form={formId} variant="primary" loading={saving}>{m.ui_save()}</Button>
  {/snippet}
</Dialog>
