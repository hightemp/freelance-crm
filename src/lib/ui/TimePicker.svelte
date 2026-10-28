<script lang="ts">
  import { i18n } from "$lib/i18n/locale.svelte";
  import { m } from "$lib/paraglide/messages";
  import TextField from "./TextField.svelte";
  import { formatTimeInput, parseTimeInput } from "./date";

  let {
    label,
    value = $bindable(null),
    step = 15,
    hint,
    error,
    required = false,
  }: {
    label: string;
    /** `HH:MM`, 24-hour. */
    value?: string | null;
    /** Minutes added/removed by ArrowUp/ArrowDown. */
    step?: number;
    hint?: string;
    error?: string;
    required?: boolean;
  } = $props();

  const format = (v: string | null) => (v ? formatTimeInput(v, i18n.locale) : "");
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
    const parsed = parseTimeInput(text);
    invalid = parsed === null;
    if (parsed) {
      value = parsed;
      text = format(parsed);
    }
  }

  function onkeydown(event: KeyboardEvent) {
    if (event.key !== "ArrowUp" && event.key !== "ArrowDown") return;
    event.preventDefault();
    const base = parseTimeInput(text) ?? value ?? "00:00";
    const [h, mm] = base.split(":").map(Number);
    const delta = event.key === "ArrowUp" ? step : -step;
    const minutes = (((h * 60 + mm + delta) % 1440) + 1440) % 1440;
    value = `${String(Math.floor(minutes / 60)).padStart(2, "0")}:${String(minutes % 60).padStart(2, "0")}`;
    text = format(value);
    invalid = false;
  }
</script>

<TextField
  {label}
  {hint}
  {required}
  bind:value={text}
  autocomplete="off"
  error={invalid ? m.error_validation_format() : error}
  onfocus={() => (focused = true)}
  onblur={commit}
  {onkeydown}
/>
