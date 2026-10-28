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
    /** Short text shown inside the field on the right, e.g. a currency code. */
    suffix?: string;
  }

  let {
    label,
    value = $bindable(""),
    type = "text",
    hint,
    error,
    required = false,
    suffix,
    ...rest
  }: Props = $props();

  const id = $props.id();
</script>

<FieldShell {id} {label} {hint} {error} required={required ?? false}>
  <div class="relative">
    <input
      {...rest}
      {id}
      {type}
      bind:value
      aria-invalid={error ? "true" : "false"}
      aria-required={required || undefined}
      aria-describedby={describedBy(id, hint, error)}
      class="h-8 {inputClass}"
      class:pr-12={suffix}
    />
    {#if suffix}
      <span
        class="pointer-events-none absolute inset-y-0 right-2.5 flex items-center text-xs text-muted"
      >
        {suffix}
      </span>
    {/if}
  </div>
</FieldShell>
