<script lang="ts">
  import { i18n } from "$lib/i18n/locale.svelte";
  import { m } from "$lib/paraglide/messages";
  import CalendarGrid from "./CalendarGrid.svelte";
  import Popover from "./Popover.svelte";
  import TextField from "./TextField.svelte";
  import { formatDateInput, parseDateInput, todayIso } from "./date";

  let {
    label,
    value = $bindable(null),
    min,
    max,
    weekStart = 1,
    today = todayIso(),
    hint,
    error,
    required = false,
  }: {
    label: string;
    /** ISO date `YYYY-MM-DD`. */
    value?: string | null;
    min?: string;
    max?: string;
    weekStart?: number;
    today?: string;
    hint?: string;
    error?: string;
    required?: boolean;
  } = $props();

  const format = (v: string | null) => (v ? formatDateInput(v, i18n.locale) : "");
  let text = $state("");
  let problem = $state<"format" | "range" | null>(null);
  let open = $state(false);
  let focused = false;

  $effect(() => {
    const next = format(value);
    if (!focused) text = next;
  });

  function commit() {
    focused = false;
    if (text.trim() === "") {
      problem = null;
      value = null;
      return;
    }
    const parsed = parseDateInput(text, i18n.locale);
    if (!parsed) problem = "format";
    else if ((min && parsed < min) || (max && parsed > max)) problem = "range";
    else {
      problem = null;
      value = parsed;
      text = format(parsed);
    }
  }

  function pick(date: string) {
    problem = null;
    value = date;
    text = format(date);
    open = false;
  }

  let shownError = $derived(
    problem === "format"
      ? m.error_validation_format()
      : problem === "range"
        ? m.error_validation_range()
        : error,
  );
</script>

<div class="flex items-start gap-1">
  <div class="flex-1">
    <TextField
      {label}
      {hint}
      {required}
      bind:value={text}
      autocomplete="off"
      error={shownError}
      onfocus={() => (focused = true)}
      onblur={commit}
    />
  </div>
  <div class="pt-6">
    <Popover {label} bind:open placement="bottom-end">
      {#snippet trigger(props)}
        <button
          {...props}
          type="button"
          aria-label={m.ui_choose_date()}
          class="grid size-8 place-items-center rounded-[var(--radius)] border border-border
            bg-bg text-muted hover:bg-surface-hover hover:text-fg"
        >
          <svg width="14" height="14" viewBox="0 0 16 16" aria-hidden="true" fill="none">
            <rect x="2" y="3" width="12" height="11" rx="1.5" stroke="currentColor" />
            <path d="M2 6.5h12M5 1.5v3M11 1.5v3" stroke="currentColor" />
          </svg>
        </button>
      {/snippet}
      <CalendarGrid selected={value} {today} {weekStart} {min} {max} autofocus onselect={pick} />
    </Popover>
  </div>
</div>
