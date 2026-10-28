<script lang="ts">
  import type { HTMLInputAttributes } from "svelte/elements";
  import FieldShell from "./FieldShell.svelte";
  import { describedBy, inputClass } from "./field";

  // Only text-like types: numbers, dates, colors etc. have dedicated components.
  type TextType = "text" | "email" | "password" | "search" | "url" | "tel";

  interface Props extends Omit<HTMLInputAttributes, "type" | "value"> {
    label: string;
    value?: string;
    type?: TextType;
    hint?: string;
    error?: string;
  }

  let {
    label,
    value = $bindable(""),
    type = "text",
    hint,
    error,
    required = false,
    ...rest
  }: Props = $props();

  const id = $props.id();
</script>

<FieldShell {id} {label} {hint} {error} required={required ?? false}>
  <input
    {...rest}
    {id}
    {type}
    bind:value
    aria-invalid={error ? "true" : "false"}
    aria-required={required || undefined}
    aria-describedby={describedBy(id, hint, error)}
    class="h-8 {inputClass}"
  />
</FieldShell>
