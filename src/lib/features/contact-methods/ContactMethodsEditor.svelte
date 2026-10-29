<script lang="ts">
  import Plus from "@lucide/svelte/icons/plus";
  import X from "@lucide/svelte/icons/x";
  import type { ContactMethodKind } from "$lib/api/bindings";
  import { codeMessage } from "$lib/i18n/errors";
  import { m } from "$lib/paraglide/messages";
  import Button from "$lib/ui/Button.svelte";
  import Select from "$lib/ui/Select.svelte";
  import TextField from "$lib/ui/TextField.svelte";
  import { METHOD_KINDS, methodLabel, type MethodDraft } from "./methods";

  let {
    methods = $bindable([]),
    errors = {},
    onremove,
  }: {
    methods?: MethodDraft[];
    /** Error codes by method index. */
    errors?: Record<number, string>;
    onremove?: (index: number) => void;
  } = $props();

  let options = $derived(METHOD_KINDS.map((k) => ({ value: k, label: methodLabel(k) })));

  function remove(index: number) {
    methods.splice(index, 1);
    onremove?.(index);
  }
</script>

<fieldset class="flex flex-col gap-3">
  <legend class="mb-3 text-base font-semibold text-fg">{m.contact_methods()}</legend>
  {#each methods as method, i (method)}
    <div class="grid grid-cols-[10rem_1fr_10rem_auto] items-start gap-2">
      <Select
        label={m.contact_method_kind()}
        {options}
        bind:value={() => method.kind, (v) => (method.kind = v as ContactMethodKind)}
      />
      <TextField
        label={m.contact_method_value()}
        bind:value={method.value}
        error={errors[i] ? codeMessage(errors[i]) : undefined}
      />
      <TextField label={m.contact_method_label()} bind:value={method.label} />
      <div class="pt-6">
        <Button
          variant="ghost"
          aria-label={m.contact_method_remove({ index: i + 1 })}
          onclick={() => remove(i)}
        >
          <X class="size-4" aria-hidden="true" />
        </Button>
      </div>
    </div>
  {/each}
  <div>
    <Button variant="ghost" onclick={() => methods.push({ kind: "email", value: "", label: "" })}>
      <Plus class="size-4" aria-hidden="true" />
      {m.contact_method_add()}
    </Button>
  </div>
</fieldset>
