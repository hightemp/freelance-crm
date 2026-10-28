<script lang="ts">
  import type { HTMLTextareaAttributes } from "svelte/elements";
  import FieldShell from "./FieldShell.svelte";
  import { describedBy, inputClass } from "./field";

  interface Props extends Omit<HTMLTextareaAttributes, "value"> {
    label: string;
    value?: string;
    hint?: string;
    error?: string;
  }

  let {
    label,
    value = $bindable(""),
    hint,
    error,
    required = false,
    rows = 4,
    ...rest
  }: Props = $props();

  const id = $props.id();
</script>

<FieldShell {id} {label} {hint} {error} required={required ?? false}>
  <textarea
    {...rest}
    {id}
    {rows}
    bind:value
    aria-invalid={error ? "true" : "false"}
    aria-required={required || undefined}
    aria-describedby={describedBy(id, hint, error)}
    class="py-1.5 {inputClass}"></textarea>
</FieldShell>
