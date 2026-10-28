<script lang="ts">
  import { rovingTarget } from "./roving";

  interface Option {
    value: string;
    label: string;
    disabled?: boolean;
  }

  let {
    label,
    options,
    value = $bindable(""),
  }: { label: string; options: Option[]; value?: string } = $props();

  const id = $props.id();
  const buttons: HTMLButtonElement[] = $state([]);

  let selectedIndex = $derived(options.findIndex((o) => o.value === value));
  // Roving tabindex: the selected radio, or the first enabled one.
  let tabbableIndex = $derived(
    selectedIndex >= 0 ? selectedIndex : options.findIndex((o) => !o.disabled),
  );

  function onkeydown(event: KeyboardEvent, index: number) {
    const target = rovingTarget(event.key, options, index);
    if (target === null) return;
    event.preventDefault();
    value = options[target].value;
    buttons[target]?.focus();
  }
</script>

<div role="radiogroup" aria-labelledby="{id}-label" class="flex flex-col gap-1.5">
  <span id="{id}-label" class="text-sm font-medium text-fg">{label}</span>
  {#each options as option, index (option.value)}
    <button
      bind:this={buttons[index]}
      type="button"
      role="radio"
      aria-checked={option.value === value}
      disabled={option.disabled}
      tabindex={index === tabbableIndex ? 0 : -1}
      onclick={() => (value = option.value)}
      onkeydown={(e) => onkeydown(e, index)}
      class="group inline-flex items-center gap-2 text-left text-sm text-fg
        disabled:cursor-not-allowed disabled:opacity-50"
    >
      <span
        class="grid size-4 place-items-center rounded-full border border-border bg-bg
          group-aria-checked:border-accent"
        aria-hidden="true"
      >
        <span class="size-2 rounded-full bg-accent opacity-0 group-aria-checked:opacity-100"></span>
      </span>
      {option.label}
    </button>
  {/each}
</div>
