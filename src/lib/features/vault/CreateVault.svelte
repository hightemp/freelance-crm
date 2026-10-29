<script lang="ts">
  import { commands } from "$lib/api/bindings";
  import { AppError, call } from "$lib/api/client";
  import { codeMessage, errorMessage } from "$lib/i18n/errors";
  import { m } from "$lib/paraglide/messages";
  import Button from "$lib/ui/Button.svelte";
  import Progress from "$lib/ui/Progress.svelte";
  import TextField from "$lib/ui/TextField.svelte";
  import { toast } from "$lib/ui/toast.svelte";
  import { passwordStrength, type Strength } from "./strength";

  let { oncreated }: { oncreated: () => void } = $props();

  const MIN_LENGTH = 8;
  let password = $state("");
  let confirm = $state("");
  let strength = $state<Strength | null>(null);
  let errors = $state<{ password?: string; confirm?: string }>({});
  let busy = $state(false);

  let labels = $derived([
    m.vault_strength_0(),
    m.vault_strength_1(),
    m.vault_strength_2(),
    m.vault_strength_3(),
    m.vault_strength_4(),
  ]);

  // Rate the password as it is typed; stale answers are ignored.
  let latest = 0;
  $effect(() => {
    const value = password;
    const request = ++latest;
    if (!value) {
      strength = null;
      return;
    }
    void passwordStrength(value).then((s) => {
      if (request === latest) strength = s;
    });
  });

  async function create(event: SubmitEvent) {
    event.preventDefault();
    errors = {};
    if ([...password].length < MIN_LENGTH) {
      errors.password = codeMessage("validation.min_length", { min: MIN_LENGTH });
    } else if (password !== confirm) {
      errors.confirm = codeMessage("validation.mismatch");
    }
    if (errors.password || errors.confirm) return;
    busy = true;
    try {
      await call(commands.vaultInit(password));
      password = "";
      confirm = "";
      oncreated();
    } catch (err) {
      if (err instanceof AppError && err.params.field === "password") {
        errors.password = errorMessage(err);
      } else {
        toast.show({ kind: "error", message: errorMessage(err) });
      }
    } finally {
      busy = false;
    }
  }
</script>

<form class="flex max-w-md flex-col gap-4" novalidate onsubmit={create}>
  <h2 class="text-base font-semibold text-fg">{m.vault_create_title()}</h2>
  <p class="text-sm text-muted">{m.vault_create_hint()}</p>
  <TextField
    label={m.vault_password()}
    type="password"
    autocomplete="new-password"
    required
    bind:value={password}
    error={errors.password}
  />
  {#if strength}
    <div class="flex flex-col gap-1">
      <Progress label={m.vault_strength()} value={strength.score} max={4} />
      <p class="text-xs text-muted">{labels[strength.score]}</p>
      {#if strength.score < 3}
        <p class="text-xs text-danger">{m.vault_weak_warning()}</p>
      {/if}
    </div>
  {/if}
  <TextField
    label={m.vault_password_confirm()}
    type="password"
    autocomplete="new-password"
    required
    bind:value={confirm}
    error={errors.confirm}
  />
  <p class="text-sm text-fg">{m.vault_no_recovery_warning()}</p>
  <div>
    <Button type="submit" variant="primary" loading={busy}>{m.vault_create()}</Button>
  </div>
</form>
