<script lang="ts">
  import { i18n } from "$lib/i18n/locale.svelte";
  import { m } from "$lib/paraglide/messages";
  import TextField from "./TextField.svelte";
  import { formatNumber, parseNumber } from "./number";

  let {
    label,
    value = $bindable(null),
    min = -Infinity,
    max = Infinity,
    step = 1,
    hint,
    error,
    required = false,
  }: {
    label: string;
    value?: number | null;
    min?: number;
    max?: number;
    step?: number;
    hint?: string;
    error?: string;
    required?: boolean;
  } = $props();

  const format = (v: number | null) => (v === null ? "" : formatNumber(v, i18n.locale));
  let text = $state(format(value));
  let invalid = $state(false);
  let focused = false;
  /** Value when editing started; restored if the input ends up invalid. */
  let before: number | null = null;

  // Reflect external value / locale changes while the user is not editing.
  $effect(() => {
    const next = format(value);
    if (!focused) text = next;
  });

  const decimals = (n: number) => (String(n).split(".")[1] ?? "").length;
  const clamp = (n: number) =>
    Number(Math.min(Math.max(n, min), max).toFixed(Math.max(decimals(step), decimals(n))));

  function commit() {
    focused = false;
    if (text.trim() === "") {
      invalid = false;
      value = null;
      return;
    }
    const parsed = parseNumber(text, i18n.locale);
    invalid = parsed === null;
    value = parsed === null ? before : clamp(parsed);
    if (!invalid) text = format(value);
  }

  /** Keeps `value` current while typing, so Enter-submitted forms see it;
   *  formatting and the invalid state wait for blur. */
  function sync() {
    if (text.trim() === "") value = null;
    else {
      const parsed = parseNumber(text, i18n.locale);
      if (parsed !== null) value = clamp(parsed);
    }
  }

  function onkeydown(event: KeyboardEvent) {
    if (event.key !== "ArrowUp" && event.key !== "ArrowDown") return;
    event.preventDefault();
    const base = parseNumber(text, i18n.locale) ?? value ?? 0;
    value = clamp(base + (event.key === "ArrowUp" ? step : -step));
    text = format(value);
    invalid = false;
  }
</script>

<TextField
  {label}
  {hint}
  {required}
  bind:value={text}
  inputmode="decimal"
  autocomplete="off"
  error={invalid ? m.error_validation_format() : error}
  onfocus={() => {
    focused = true;
    before = value;
  }}
  oninput={sync}
  onblur={commit}
  {onkeydown}
/>
