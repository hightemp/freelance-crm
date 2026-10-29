<script lang="ts">
  import RefreshCw from "@lucide/svelte/icons/refresh-cw";
  import { commands, type Generated } from "$lib/api/bindings";
  import { call } from "$lib/api/client";
  import { errorMessage } from "$lib/i18n/errors";
  import { i18n } from "$lib/i18n/locale.svelte";
  import { m } from "$lib/paraglide/messages";
  import Button from "$lib/ui/Button.svelte";
  import Checkbox from "$lib/ui/Checkbox.svelte";
  import Dialog from "$lib/ui/Dialog.svelte";
  import NumberField from "$lib/ui/NumberField.svelte";
  import RadioGroup from "$lib/ui/RadioGroup.svelte";
  import TextField from "$lib/ui/TextField.svelte";
  import { toast } from "$lib/ui/toast.svelte";

  let { onuse, oncancel }: { onuse: (value: string) => void; oncancel: () => void } = $props();

  let mode = $state<"password" | "passphrase">("password");
  let length = $state<number | null>(20);
  let lowercase = $state(true);
  let uppercase = $state(true);
  let digits = $state(true);
  let symbols = $state(true);
  let excludeSimilar = $state(false);
  let words = $state<number | null>(5);
  let separator = $state("-");
  let capitalize = $state(false);
  let result = $state<Generated | null>(null);
  /** Bumped to generate again with the same options. */
  let round = $state(0);

  let modes = $derived([
    { value: "password", label: m.generator_password() },
    { value: "passphrase", label: m.generator_passphrase() },
  ]);

  // Generates whenever an option changes; stale answers are ignored.
  let latest = 0;
  $effect(() => {
    void round;
    const request =
      mode === "password"
        ? commands.generatePassword({
            length: length ?? 20,
            lowercase,
            uppercase,
            digits,
            symbols,
            excludeSimilar,
          })
        : commands.generatePassphrase({ words: words ?? 5, separator, capitalize });
    const id = ++latest;
    call(request).then(
      (generated) => {
        if (id === latest) result = generated;
      },
      (err) => toast.show({ kind: "error", message: errorMessage(err) }),
    );
  });

  let entropy = $derived(
    result
      ? new Intl.NumberFormat(i18n.locale, { maximumFractionDigits: 0 }).format(
          result.entropyBits ?? 0,
        )
      : "",
  );
</script>

<Dialog open title={m.generator_title()} onclose={oncancel}>
  <div class="flex w-96 max-w-full flex-col gap-4">
    <RadioGroup
      label={m.generator_mode()}
      options={modes}
      bind:value={() => mode, (v) => (mode = v as typeof mode)}
    />
    {#if mode === "password"}
      <NumberField label={m.generator_length()} min={4} max={128} bind:value={length} />
      <Checkbox label={m.generator_lowercase()} bind:checked={lowercase} />
      <Checkbox label={m.generator_uppercase()} bind:checked={uppercase} />
      <Checkbox label={m.generator_digits()} bind:checked={digits} />
      <Checkbox label={m.generator_symbols()} bind:checked={symbols} />
      <Checkbox label={m.generator_exclude_similar()} bind:checked={excludeSimilar} />
    {:else}
      <NumberField label={m.generator_words()} min={3} max={16} bind:value={words} />
      <TextField label={m.generator_separator()} bind:value={separator} />
      <Checkbox label={m.generator_capitalize()} bind:checked={capitalize} />
    {/if}
    <div class="flex flex-col gap-1">
      <span class="text-sm font-medium text-fg">{m.generator_result()}</span>
      <div class="flex items-center gap-2">
        <code
          class="min-w-0 flex-1 rounded-[var(--radius)] bg-surface px-2 py-1.5 font-mono text-sm break-all text-fg"
          >{result?.value ?? ""}</code
        >
        <Button
          variant="ghost"
          size="sm"
          aria-label={m.generator_regenerate()}
          onclick={() => round++}
        >
          <RefreshCw class="size-4" aria-hidden="true" />
        </Button>
      </div>
      {#if result}
        <p class="text-xs text-muted">{m.generator_entropy({ bits: entropy })}</p>
      {/if}
    </div>
  </div>
  {#snippet footer()}
    <Button onclick={oncancel}>{m.ui_cancel()}</Button>
    <Button variant="primary" disabled={!result} onclick={() => result && onuse(result.value)}>
      {m.generator_use()}
    </Button>
  {/snippet}
</Dialog>
