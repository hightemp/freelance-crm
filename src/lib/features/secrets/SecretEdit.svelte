<script lang="ts">
  import { commands, type SecretPayload, type Secret_Serialize as Secret } from "$lib/api/bindings";
  import { call } from "$lib/api/client";
  import { errorMessage } from "$lib/i18n/errors";
  import { m } from "$lib/paraglide/messages";
  import Spinner from "$lib/ui/Spinner.svelte";
  import SecretForm from "./SecretForm.svelte";

  let {
    id,
    onsaved,
    oncancel,
  }: { id: string; onsaved: (secret: Secret) => void; oncancel: () => void } = $props();

  let loaded = $state<{ secret: Secret; payload: SecretPayload } | null>(null);
  let loadError = $state<string | null>(null);

  $effect(() => {
    Promise.all([call(commands.getSecret(id)), call(commands.secretPayload(id))]).then(
      ([secret, payload]) => (loaded = { secret, payload }),
      (err) => (loadError = errorMessage(err)),
    );
  });
</script>

<div class="flex flex-col gap-6">
  <h1 class="text-xl font-semibold text-fg">{m.secret_edit()}</h1>
  {#if loadError}
    <p class="text-sm text-danger" role="alert">{loadError}</p>
  {:else if loaded}
    <SecretForm secret={loaded.secret} payload={loaded.payload} {onsaved} {oncancel} />
  {:else}
    <div class="flex justify-center p-8"><Spinner /></div>
  {/if}
</div>
