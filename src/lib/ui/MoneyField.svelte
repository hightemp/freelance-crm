<script lang="ts">
  import { i18n } from "$lib/i18n/locale.svelte";
  import { m } from "$lib/paraglide/messages";
  import TextField from "./TextField.svelte";
  import { formatMinor, parseMinor } from "./number";

  let {
    label,
    value = $bindable(null),
    currency,
    hint,
    error,
    required = false,
  }: {
    label: string;
    /** Amount in minor units (kopecks, cents). */
    value?: number | null;
    /** ISO 4217 code. */
    currency: string;
    hint?: string;
    error?: string;
    required?: boolean;
  } = $props();

  let digits = $derived(
    new Intl.NumberFormat("en", { style: "currency", currency }).resolvedOptions()
      .maximumFractionDigits ?? 2,
  );
  const format = (v: number | null) => (v === null ? "" : formatMinor(v, i18n.locale, digits));
  let text = $state("");
  let invalid = $state(false);
  let focused = false;

  $effect(() => {
    const next = format(value);
    if (!focused) text = next;
  });

  function commit() {
    focused = false;
    if (text.trim() === "") {
      invalid = false;
      value = null;
      return;
    }
    const parsed = parseMinor(text, i18n.locale, digits);
    invalid = parsed === null;
    if (parsed === null) value = null;
    else {
      value = parsed;
      text = format(parsed);
    }
  }
</script>

<TextField
  {label}
  {hint}
  {required}
  bind:value={text}
  inputmode="decimal"
  autocomplete="off"
  suffix={currency}
  error={invalid ? m.error_validation_format() : error}
  onfocus={() => (focused = true)}
  onblur={commit}
/>
