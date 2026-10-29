<script lang="ts">
  import { resolve } from "$app/paths";
  import {
    commands,
    type Client_Serialize as Client,
    type ProjectFilter_Deserialize as ProjectFilter,
    type ProjectStatus,
    type Project_Serialize as Project,
  } from "$lib/api/bindings";
  import { call } from "$lib/api/client";
  import { errorMessage } from "$lib/i18n/errors";
  import { formatIsoDate, formatMoney } from "$lib/i18n/format";
  import { i18n } from "$lib/i18n/locale.svelte";
  import { m } from "$lib/paraglide/messages";
  import Select from "$lib/ui/Select.svelte";
  import Spinner from "$lib/ui/Spinner.svelte";
  import Switch from "$lib/ui/Switch.svelte";
  import TextField from "$lib/ui/TextField.svelte";
  import { toast } from "$lib/ui/toast.svelte";
  import { PROJECT_STATUSES, projectStatusLabel } from "./labels";

  let {
    clientId,
    searchDelay = 300,
  }: {
    /** Show only this client's projects (inside the client card). */
    clientId?: string;
    /** Delay before a search query is sent, in milliseconds. */
    searchDelay?: number;
  } = $props();

  const ALL = "all";

  let search = $state("");
  let query = $state("");
  let status = $state<string | null>(ALL);
  let client = $state<string | null>(ALL);
  let archived = $state(false);
  let projects = $state<Project[] | null>(null);
  let clients = $state<Client[]>([]);

  let filtered = $derived(query !== "" || status !== ALL || client !== ALL || archived);
  let statusOptions = $derived([
    { value: ALL, label: m.clients_status_all() },
    ...PROJECT_STATUSES.map((s) => ({ value: s, label: projectStatusLabel(s) })),
  ]);
  let clientOptions = $derived([
    { value: ALL, label: m.projects_client_all() },
    ...clients.map((c) => ({ value: c.id, label: c.displayName })),
  ]);

  const showError = (err: unknown) => toast.show({ kind: "error", message: errorMessage(err) });

  $effect(() => {
    const next = search.trim();
    const timer = setTimeout(() => (query = next), searchDelay);
    return () => clearTimeout(timer);
  });

  let latest = 0;
  $effect(() => {
    const filter: ProjectFilter = {
      query: query || null,
      status: status === ALL ? null : (status as ProjectStatus),
      clientId: clientId ?? (client === ALL ? null : client),
      archived,
    };
    const request = ++latest;
    call(commands.listProjects(filter)).then((result) => {
      if (request === latest) projects = result;
    }, showError);
  });

  $effect(() => {
    if (clientId) return;
    call(commands.listClients({ query: null, status: null, archived: false, tagIds: [] })).then(
      (result) => (clients = result),
      showError,
    );
  });

  function price(p: Project): string {
    if (p.billingType === "fixed") {
      return p.budgetMinor === null ? "" : formatMoney(p.budgetMinor, p.currency, i18n.locale);
    }
    if (p.rateMinor === null) return "";
    return m.client_rate_per_hour({ amount: formatMoney(p.rateMinor, p.currency, i18n.locale) });
  }
</script>

<div class="flex flex-col gap-4">
  <div class="flex flex-wrap items-end gap-3">
    <div class="w-64">
      <TextField label={m.projects_search()} bind:value={search} />
    </div>
    <div class="w-44">
      <Select label={m.clients_status()} options={statusOptions} bind:value={status} />
    </div>
    {#if !clientId}
      <div class="w-52">
        <Select label={m.projects_client()} options={clientOptions} bind:value={client} />
      </div>
    {/if}
    <div class="pb-1.5">
      <Switch label={m.clients_show_archived()} bind:checked={archived} />
    </div>
  </div>

  {#if projects === null}
    <div class="flex justify-center p-8"><Spinner /></div>
  {:else if projects.length === 0}
    <div
      class="flex flex-col items-center gap-2 rounded-[var(--radius)] border border-dashed border-border p-10 text-center"
    >
      {#if filtered}
        <p class="text-sm font-medium text-fg">{m.projects_no_matches()}</p>
      {:else}
        <p class="text-sm font-medium text-fg">{m.projects_empty_title()}</p>
        <p class="max-w-sm text-sm text-muted">{m.projects_empty_hint()}</p>
      {/if}
    </div>
  {:else}
    <table class="w-full border-collapse text-sm">
      <thead class="border-b border-border text-left text-muted">
        <tr>
          <th scope="col" class="px-3 py-2 font-medium">{m.projects_col_name()}</th>
          {#if !clientId}
            <th scope="col" class="px-3 py-2 font-medium">{m.projects_col_client()}</th>
          {/if}
          <th scope="col" class="px-3 py-2 font-medium">{m.projects_col_status()}</th>
          <th scope="col" class="px-3 py-2 text-right font-medium">{m.projects_col_price()}</th>
          <th scope="col" class="px-3 py-2 font-medium">{m.projects_col_deadline()}</th>
        </tr>
      </thead>
      <tbody>
        {#each projects as p (p.id)}
          <tr class="border-b border-border hover:bg-surface-hover">
            <td class="px-3 py-2 text-fg">
              <a
                href={resolve("/projects/[id]", { id: p.id })}
                class="hover:underline focus-visible:outline-2 focus-visible:outline-focus"
              >
                {p.name}
              </a>
            </td>
            {#if !clientId}
              <td class="px-3 py-2 text-fg">{p.clientName}</td>
            {/if}
            <td class="px-3 py-2 text-fg">{projectStatusLabel(p.status)}</td>
            <td class="px-3 py-2 text-right whitespace-nowrap text-fg tabular-nums">{price(p)}</td>
            <td class="px-3 py-2 whitespace-nowrap text-muted">
              {p.deadline ? formatIsoDate(p.deadline, i18n.locale) : ""}
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</div>
