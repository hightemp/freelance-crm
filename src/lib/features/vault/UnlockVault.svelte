<script lang="ts">
  import { tick } from "svelte";
  import { commands } from "$lib/api/bindings";
  import { call } from "$lib/api/client";
  import { errorMessage } from "$lib/i18n/errors";
  import { m } from "$lib/paraglide/messages";
  import Button from "$lib/ui/Button.svelte";
  import TextField from "$lib/ui/TextField.svelte";

  let { onunlocked, onforgot }: { onunlocked: () => void; onforgot: () => void } = $props();

  let password = $state("");
  let error = $state<string | undefined>();
  let busy = $state(false);
  let form: HTMLFormElement | undefined = $state();

  async function unlock(event: SubmitEvent) {
    event.preventDefault();
    busy = true;
    try {
      await call(commands.vaultUnlock(password));
      password = "";
      onunlocked();
    } catch (err) {
      // A wrong password is cleared so the next attempt starts fresh.
      error = errorMessage(err);
      password = "";
      await tick();
      form?.querySelector("input")?.focus();
    } finally {
      busy = false;
    }
  }
</script>

<form bind:this={form} class="flex max-w-md flex-col gap-4" novalidate onsubmit={unlock}>
  <h2 class="text-base font-semibold text-fg">{m.vault_unlock_title()}</h2>
  <p class="text-sm text-muted">{m.vault_unlock_hint()}</p>
  <TextField
    label={m.vault_password()}
    type="password"
    autocomplete="current-password"
    bind:value={password}
    {error}
    oninput={() => (error = undefined)}
  />
  <div class="flex flex-wrap gap-2">
    <Button type="submit" variant="primary" loading={busy}>{m.vault_unlock()}</Button>
    <Button variant="ghost" onclick={onforgot}>{m.recover_link()}</Button>
  </div>
</form>
