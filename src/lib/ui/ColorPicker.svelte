<script lang="ts">
  import { m } from "$lib/paraglide/messages";
  import Popover from "./Popover.svelte";
  import TextField from "./TextField.svelte";
  import { normalizeHex, PRESET_COLORS } from "./color";

  let { label, value = $bindable("#3b5bdb") }: { label: string; value?: string } = $props();

  let open = $state(false);
  let hex = $state(value);
  let invalid = $state(false);

  function pick(color: string) {
    value = color;
    hex = color;
    invalid = false;
  }

  function onkeydown(event: KeyboardEvent) {
    if (event.key !== "Enter") return;
    event.preventDefault();
    const normalized = normalizeHex(hex);
    invalid = normalized === null;
    if (normalized) pick(normalized);
  }
</script>

<Popover {label} bind:open>
  {#snippet trigger(props)}
    <button
      {...props}
      type="button"
      class="inline-flex h-8 items-center gap-2 rounded-[var(--radius)] border border-border bg-bg
        px-2.5 text-sm text-fg"
    >
      <span class="size-4 rounded border border-border" style:background={value}></span>
      {label}
    </button>
  {/snippet}
  <div class="flex w-56 flex-col gap-3">
    <div role="radiogroup" aria-label={m.ui_presets()} class="grid grid-cols-6 gap-1.5">
      {#each PRESET_COLORS as color (color)}
        <button
          type="button"
          role="radio"
          aria-checked={color === value}
          aria-label={color}
          onclick={() => pick(color)}
          class="size-7 rounded border border-border aria-checked:ring-2 aria-checked:ring-focus"
          style:background={color}
        ></button>
      {/each}
    </div>
    <TextField
      label={m.ui_hex()}
      bind:value={hex}
      error={invalid ? m.error_validation_format() : undefined}
      {onkeydown}
    />
  </div>
</Popover>
