<script lang="ts">
  import {
    commands,
    type ContactMethod_Serialize as ContactMethod,
    type Client_Serialize as Client,
  } from "$lib/api/bindings";
  import { call } from "$lib/api/client";
  import { errorMessage } from "$lib/i18n/errors";
  import { m } from "$lib/paraglide/messages";
  import Spinner from "$lib/ui/Spinner.svelte";
  import ClientForm from "./ClientForm.svelte";

  let {
    id,
    onsaved,
    oncancel,
  }: { id: string; onsaved: (client: Client) => void; oncancel: () => void } = $props();

  let loaded = $state<{ client: Client; methods: ContactMethod[] } | null>(null);
  let loadError = $state<string | null>(null);

  $effect(() => {
    const clientId = id;
    Promise.all([
      call(commands.getClient(clientId)),
      call(commands.listContactMethods({ type: "client", id: clientId })),
    ]).then(
      ([client, methods]) => (loaded = { client, methods }),
      (err) => (loadError = errorMessage(err)),
    );
  });
</script>

<div class="flex flex-col gap-6">
  <h1 class="text-xl font-semibold text-fg">{m.client_edit()}</h1>
  {#if loadError}
    <p class="text-sm text-danger" role="alert">{loadError}</p>
  {:else if loaded}
    <ClientForm client={loaded.client} methods={loaded.methods} {onsaved} {oncancel} />
  {:else}
    <div class="flex justify-center p-8"><Spinner /></div>
  {/if}
</div>
