<script lang="ts">
  import { resolve } from "$app/paths";
  import {
    commands,
    type SecretFilter_Deserialize as SecretFilter,
    type Secret_Serialize as Secret,
  } from "$lib/api/bindings";
  import { call } from "$lib/api/client";
  import { errorMessage } from "$lib/i18n/errors";
  import { formatIsoDate } from "$lib/i18n/format";
  import { i18n } from "$lib/i18n/locale.svelte";
  import { m } from "$lib/paraglide/messages";
  import Spinner from "$lib/ui/Spinner.svelte";
  import TextField from "$lib/ui/TextField.svelte";
  import { toast } from "$lib/ui/toast.svelte";
  import { kindLabel } from "./labels";

  let {
    clientId,
    projectId,
    searchDelay = 300,
  }: {
    /** Only secrets of this client (and its projects) or project. */
    clientId?: string;
    projectId?: string;
    searchDelay?: number;
  } = $props();

  let search = $state("");
  let query = $state("");
  let secrets = $state<Secret[] | null>(null);

  $effect(() => {
    const next = search.trim();
    const timer = setTimeout(() => (query = next), searchDelay);
    return () => clearTimeout(timer);
  });

  let latest = 0;
  $effect(() => {
    const filter: SecretFilter = {
      query: query || null,
      kind: null,
      clientId: clientId ?? null,
      projectId: projectId ?? null,
    };
    const request = ++latest;
    call(commands.listSecrets(filter)).then(
      (result) => {
        if (request === latest) secrets = result;
      },
      (err) => toast.show({ kind: "error", message: errorMessage(err) }),
    );
  });

  function where(s: Secret): string {
    if (!s.clientName) return "";
    return s.projectName ? `${s.clientName} · ${s.projectName}` : s.clientName;
  }
</script>

<div class="flex flex-col gap-4">
  <div class="w-64">
    <TextField label={m.secrets_search()} bind:value={search} />
  </div>
  {#if secrets === null}
    <div class="flex justify-center p-8"><Spinner /></div>
  {:else if secrets.length === 0}
    <div
      class="flex flex-col items-center gap-2 rounded-[var(--radius)] border border-dashed border-border p-10 text-center"
    >
      {#if query}
        <p class="text-sm font-medium text-fg">{m.secrets_no_matches()}</p>
      {:else}
        <p class="text-sm font-medium text-fg">{m.secrets_empty_title()}</p>
        <p class="max-w-sm text-sm text-muted">{m.secrets_empty_hint()}</p>
      {/if}
    </div>
  {:else}
    <ul class="flex flex-col divide-y divide-border rounded-[var(--radius)] border border-border">
      {#each secrets as s (s.id)}
        <li class="flex items-center gap-3 px-3 py-2 text-sm">
          <a
            href={resolve("/vault/[id]", { id: s.id })}
            class="min-w-0 flex-1 truncate text-fg hover:underline focus-visible:outline-2 focus-visible:outline-focus"
          >
            {s.title}
          </a>
          <span class="w-36 shrink-0 text-xs text-muted">{kindLabel(s.kind)}</span>
          <span class="w-48 shrink-0 truncate text-xs text-muted">{where(s)}</span>
          <span class="w-40 shrink-0 text-xs text-muted">
            {s.expiresOn ? m.secret_expires({ date: formatIsoDate(s.expiresOn, i18n.locale) }) : ""}
          </span>
        </li>
      {/each}
    </ul>
  {/if}
</div>
