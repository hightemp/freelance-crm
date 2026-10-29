<script lang="ts">
  import { onMount } from "svelte";
  import { resolve } from "$app/paths";
  import ArrowDown from "@lucide/svelte/icons/arrow-down";
  import ArrowUp from "@lucide/svelte/icons/arrow-up";
  import Building2 from "@lucide/svelte/icons/building-2";
  import User from "@lucide/svelte/icons/user";
  import {
    commands,
    type ClientFilter_Deserialize as ClientFilter,
    type ClientStatus,
    type Client_Serialize as Client,
    type Tag,
  } from "$lib/api/bindings";
  import { call } from "$lib/api/client";
  import { appSettings } from "$lib/app/settings.svelte";
  import { errorMessage } from "$lib/i18n/errors";
  import { formatDate, formatMoney } from "$lib/i18n/format";
  import { i18n } from "$lib/i18n/locale.svelte";
  import { m } from "$lib/paraglide/messages";
  import Button from "$lib/ui/Button.svelte";
  import Checkbox from "$lib/ui/Checkbox.svelte";
  import Popover from "$lib/ui/Popover.svelte";
  import Select from "$lib/ui/Select.svelte";
  import Spinner from "$lib/ui/Spinner.svelte";
  import Switch from "$lib/ui/Switch.svelte";
  import TextField from "$lib/ui/TextField.svelte";
  import { toast } from "$lib/ui/toast.svelte";
  import TagChip from "../tags/TagChip.svelte";
  import { CLIENT_STATUSES, statusLabel } from "./status";

  /** Delay before a search query is sent, in milliseconds. */
  let { searchDelay = 300 }: { searchDelay?: number } = $props();

  type SortKey = "name" | "status" | "updated";
  const ALL = "all";

  let search = $state("");
  let query = $state("");
  let status = $state<string | null>(ALL);
  let archived = $state(false);
  let tagIds = $state<string[]>([]);
  let sort = $state<{ key: SortKey; desc: boolean }>({ key: "name", desc: false });

  /** `null` until the first response arrives. */
  let clients = $state<Client[] | null>(null);
  let tags = $state<Tag[]>([]);

  let tagsById = $derived(new Map(tags.map((t) => [t.id, t])));
  let filtered = $derived(query !== "" || status !== ALL || archived || tagIds.length > 0);
  let statusOptions = $derived([
    { value: ALL, label: m.clients_status_all() },
    ...CLIENT_STATUSES.map((s) => ({ value: s, label: statusLabel(s) })),
  ]);

  let sorted = $derived.by(() => {
    if (!clients) return [];
    const collator = new Intl.Collator(i18n.locale, { sensitivity: "base", numeric: true });
    const compare: Record<SortKey, (a: Client, b: Client) => number> = {
      name: (a, b) => collator.compare(a.displayName, b.displayName),
      status: (a, b) => CLIENT_STATUSES.indexOf(a.status) - CLIENT_STATUSES.indexOf(b.status),
      updated: (a, b) => a.updatedAt - b.updatedAt,
    };
    const dir = sort.desc ? -1 : 1;
    return [...clients].sort((a, b) => dir * compare[sort.key](a, b) || compare.name(a, b));
  });

  // Search is debounced; other filters apply immediately.
  $effect(() => {
    const next = search.trim();
    const timer = setTimeout(() => (query = next), searchDelay);
    return () => clearTimeout(timer);
  });

  let latest = 0;
  $effect(() => {
    const filter: ClientFilter = {
      query: query || null,
      status: status === ALL ? null : (status as ClientStatus),
      archived,
      tagIds: [...tagIds],
    };
    const request = ++latest;
    call(commands.listClients(filter)).then(
      (result) => {
        // Responses may arrive out of order; only the latest one counts.
        if (request === latest) clients = result;
      },
      (err) => toast.show({ kind: "error", message: errorMessage(err) }),
    );
  });

  onMount(() => {
    call(commands.listTags()).then(
      (result) => (tags = result),
      (err) => toast.show({ kind: "error", message: errorMessage(err) }),
    );
  });

  function toggleTag(id: string, on: boolean) {
    tagIds = on ? [...tagIds, id] : tagIds.filter((t) => t !== id);
  }

  function resetFilters() {
    search = "";
    query = "";
    status = ALL;
    archived = false;
    tagIds = [];
  }

  function sortBy(key: SortKey) {
    sort = { key, desc: sort.key === key ? !sort.desc : false };
  }

  function ariaSort(key: SortKey) {
    if (sort.key !== key) return undefined;
    return sort.desc ? "descending" : "ascending";
  }

  function rate(c: Client): string {
    if (c.defaultRateMinor === null) return "";
    const currency = c.defaultCurrency ?? appSettings.value.defaultCurrency;
    const amount = formatMoney(c.defaultRateMinor, currency, i18n.locale);
    return m.client_rate_per_hour({ amount });
  }
</script>

{#snippet header(key: SortKey, label: string)}
  <th scope="col" aria-sort={ariaSort(key)} class="px-3 py-2 text-left font-medium">
    <button
      type="button"
      class="inline-flex items-center gap-1 rounded hover:text-fg focus-visible:outline-2 focus-visible:outline-focus"
      onclick={() => sortBy(key)}
    >
      {label}
      {#if sort.key === key}
        {@const Icon = sort.desc ? ArrowDown : ArrowUp}
        <Icon class="size-3.5" aria-hidden="true" />
      {/if}
    </button>
  </th>
{/snippet}

<div class="flex flex-col gap-4">
  <div class="flex flex-wrap items-end gap-3">
    <div class="w-64">
      <TextField label={m.clients_search()} bind:value={search} />
    </div>
    <div class="w-44">
      <Select label={m.clients_status()} options={statusOptions} bind:value={status} />
    </div>
    <Popover label={m.clients_tags()}>
      {#snippet trigger(props)}
        <Button {...props}>
          {tagIds.length ? m.clients_tags_selected({ count: tagIds.length }) : m.clients_tags()}
        </Button>
      {/snippet}
      <div class="flex max-h-64 min-w-48 flex-col gap-2 overflow-auto">
        {#each tags as tag (tag.id)}
          <Checkbox
            label={tag.name}
            bind:checked={() => tagIds.includes(tag.id), (on) => toggleTag(tag.id, on)}
          />
        {:else}
          <p class="text-sm text-muted">{m.clients_no_tags()}</p>
        {/each}
      </div>
    </Popover>
    <div class="pb-1.5">
      <Switch label={m.clients_show_archived()} bind:checked={archived} />
    </div>
  </div>

  {#if clients === null}
    <div class="flex justify-center p-8"><Spinner /></div>
  {:else if clients.length === 0}
    <div
      class="flex flex-col items-center gap-2 rounded-[var(--radius)] border border-dashed border-border p-10 text-center"
    >
      {#if filtered}
        <p class="text-sm font-medium text-fg">{m.clients_no_matches()}</p>
        <Button variant="ghost" onclick={resetFilters}>{m.clients_reset_filters()}</Button>
      {:else}
        <p class="text-sm font-medium text-fg">{m.clients_empty_title()}</p>
        <p class="max-w-sm text-sm text-muted">{m.clients_empty_hint()}</p>
      {/if}
    </div>
  {:else}
    <table class="w-full border-collapse text-sm">
      <thead class="border-b border-border text-muted">
        <tr>
          {@render header("name", m.clients_col_name())}
          {@render header("status", m.clients_col_status())}
          <th scope="col" class="px-3 py-2 text-left font-medium">{m.clients_col_tags()}</th>
          <th scope="col" class="px-3 py-2 text-right font-medium">{m.clients_col_rate()}</th>
          {@render header("updated", m.clients_col_updated())}
        </tr>
      </thead>
      <tbody>
        {#each sorted as c (c.id)}
          {@const KindIcon = c.kind === "person" ? User : Building2}
          <tr class="border-b border-border hover:bg-surface-hover">
            <td class="px-3 py-2 text-fg">
              <span class="inline-flex items-center gap-2">
                <KindIcon
                  class="size-4 shrink-0 text-muted"
                  aria-label={c.kind === "person"
                    ? m.client_kind_person()
                    : m.client_kind_company()}
                />
                <a
                  href={resolve("/clients/[id]", { id: c.id })}
                  class="truncate hover:underline focus-visible:outline-2 focus-visible:outline-focus"
                >
                  {c.displayName}
                </a>
              </span>
            </td>
            <td class="px-3 py-2 text-fg">{statusLabel(c.status)}</td>
            <td class="px-3 py-2">
              <span class="flex flex-wrap gap-1">
                {#each c.tagIds as id (id)}
                  {@const tag = tagsById.get(id)}
                  {#if tag}<TagChip name={tag.name} color={tag.color} />{/if}
                {/each}
              </span>
            </td>
            <td class="px-3 py-2 text-right whitespace-nowrap text-fg tabular-nums">{rate(c)}</td>
            <td class="px-3 py-2 whitespace-nowrap text-muted">
              {formatDate(c.updatedAt, i18n.locale)}
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</div>
