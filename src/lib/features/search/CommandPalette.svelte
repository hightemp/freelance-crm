<script lang="ts">
  import { goto } from "$app/navigation";
  import { resolve } from "$app/paths";
  import { commands, type SearchHit, type SearchKind } from "$lib/api/bindings";
  import { call } from "$lib/api/client";
  import { errorMessage } from "$lib/i18n/errors";
  import { m } from "$lib/paraglide/messages";
  import Dialog from "$lib/ui/Dialog.svelte";
  import { toast } from "$lib/ui/toast.svelte";
  import { stopTimer, timer, timerPanel } from "../time/timer.svelte";
  import { hitHref } from "./target";

  // Global search and command palette (FR-SR-1, 4): Ctrl/Cmd+K (or
  // Ctrl/Cmd+F) from any screen; arrows move, Enter opens or runs.

  /** Opened by the layout for Ctrl/Cmd+F. */
  let { open = $bindable(false) }: { open?: boolean } = $props();

  const id = $props.id();
  const LIMIT = 20;

  interface Command {
    key: string;
    label: string;
    run: () => unknown;
  }

  type Entry = { type: "command"; command: Command } | { type: "hit"; hit: SearchHit };

  let query = $state("");
  let hits = $state<SearchHit[]>([]);
  let activeIndex = $state(0);
  /** Answers to older queries are dropped. */
  let asked = 0;

  // Paths here are resolved by the callers (and by `hitHref`).
  // eslint-disable-next-line svelte/no-navigation-without-resolve
  const go = (path: string) => () => goto(path);
  let allCommands = $derived<Command[]>([
    { key: "new-task", label: m.palette_new_task(), run: go(resolve("/tasks/new")) },
    { key: "new-client", label: m.palette_new_client(), run: go(resolve("/clients/new")) },
    { key: "new-project", label: m.palette_new_project(), run: go(resolve("/projects/new")) },
    timer.running
      ? { key: "stop-timer", label: m.timer_stop(), run: () => stopTimer() }
      : { key: "start-timer", label: m.timer_start(), run: () => (timerPanel.open = true) },
    { key: "lock-vault", label: m.palette_lock_vault(), run: () => commands.vaultLock() },
    {
      key: "go-calendar",
      label: m.palette_go({ page: m.nav_calendar() }),
      run: go(resolve("/calendar")),
    },
    { key: "go-time", label: m.palette_go({ page: m.nav_time() }), run: go(resolve("/time")) },
    {
      key: "go-finance",
      label: m.palette_go({ page: m.nav_finance() }),
      run: go(resolve("/finance")),
    },
    {
      key: "go-settings",
      label: m.palette_go({ page: m.nav_settings() }),
      run: go(resolve("/settings")),
    },
  ]);

  const fold = (s: string) => s.toLocaleLowerCase().replaceAll("ё", "е");
  let matching = $derived.by(() => {
    const words = fold(query).split(/\s+/).filter(Boolean);
    return allCommands.filter((c) => words.every((w) => fold(c.label).includes(w)));
  });
  let entries = $derived<Entry[]>([
    ...matching.map((command) => ({ type: "command" as const, command })),
    ...(query.trim() ? hits.map((hit) => ({ type: "hit" as const, hit })) : []),
  ]);
  let active = $derived(entries.length ? Math.min(activeIndex, entries.length - 1) : -1);

  function onwindowkeydown(event: KeyboardEvent) {
    const mod = event.ctrlKey || event.metaKey;
    if (!mod || event.altKey || event.shiftKey || event.key.toLowerCase() !== "k") return;
    event.preventDefault();
    open = true;
  }

  // A fresh start each time it opens.
  $effect(() => {
    if (!open) return;
    query = "";
    hits = [];
    activeIndex = 0;
  });

  $effect(() => {
    const text = query.trim();
    const ticket = ++asked;
    activeIndex = 0;
    if (!text) {
      hits = [];
      return;
    }
    const timeout = setTimeout(() => {
      call(commands.search(text, LIMIT))
        .then((found) => {
          if (ticket === asked) hits = found;
        })
        .catch((err) => toast.show({ kind: "error", message: errorMessage(err) }));
    }, 120);
    return () => clearTimeout(timeout);
  });

  async function choose(entry: Entry) {
    open = false;
    try {
      if (entry.type === "command") await entry.command.run();
      else await go(hitHref(entry.hit))();
    } catch (err) {
      toast.show({ kind: "error", message: errorMessage(err) });
    }
  }

  function oninputkeydown(event: KeyboardEvent) {
    const n = entries.length;
    if (event.key === "ArrowDown" && n) {
      event.preventDefault();
      activeIndex = (active + 1) % n;
    } else if (event.key === "ArrowUp" && n) {
      event.preventDefault();
      activeIndex = (active - 1 + n) % n;
    } else if (event.key === "Enter" && active >= 0) {
      event.preventDefault();
      void choose(entries[active]);
    }
  }

  function kindLabel(kind: SearchKind): string {
    switch (kind) {
      case "client":
        return m.search_kind_client();
      case "contact":
        return m.search_kind_contact();
      case "project":
        return m.search_kind_project();
      case "task":
        return m.search_kind_task();
      case "event":
        return m.search_kind_event();
      case "note":
        return m.search_kind_note();
      case "interaction":
        return m.search_kind_interaction();
      case "field":
        return m.search_kind_field();
      case "file":
        return m.search_kind_file();
      case "secret":
        return m.search_kind_secret();
      case "invoice":
        return m.search_kind_invoice();
    }
  }

  /** The snippet split into plain and matched parts (`[match]`). */
  function parts(snippet: string): { text: string; match: boolean }[] {
    return snippet
      .split(/(\[[^\]]*\])/)
      .filter(Boolean)
      .map((text) =>
        text.startsWith("[") && text.endsWith("]")
          ? { text: text.slice(1, -1), match: true }
          : { text, match: false },
      );
  }

  const optionId = (index: number) => `${id}-option-${index}`;
</script>

<svelte:window onkeydown={onwindowkeydown} />

{#if open}
  <Dialog open title={m.palette_title()} onclose={() => (open = false)}>
    <div class="flex w-[36rem] max-w-full flex-col gap-2">
      <input
        data-autofocus
        type="text"
        role="combobox"
        aria-label={m.palette_input()}
        aria-expanded="true"
        aria-controls="{id}-list"
        aria-autocomplete="list"
        aria-activedescendant={active >= 0 ? optionId(active) : undefined}
        autocomplete="off"
        spellcheck="false"
        placeholder={m.palette_placeholder()}
        class="h-10 w-full rounded-md border border-border bg-bg px-3 text-sm text-fg outline-none focus:border-focus"
        bind:value={query}
        onkeydown={oninputkeydown}
      />
      <ul
        id="{id}-list"
        role="listbox"
        aria-label={m.palette_results()}
        class="flex max-h-96 flex-col overflow-y-auto"
      >
        {#each entries as entry, index (entry.type === "command" ? entry.command.key : `${entry.hit.kind}:${entry.hit.id}`)}
          <!-- Keyboard use goes through the combobox input. -->
          <!-- svelte-ignore a11y_click_events_have_key_events -->
          <li
            id={optionId(index)}
            role="option"
            aria-selected={index === active}
            class="flex cursor-pointer flex-col gap-0.5 rounded-md px-3 py-2 text-sm {index ===
            active
              ? 'bg-accent/15'
              : 'hover:bg-surface-hover'}"
            onmousedown={(e) => e.preventDefault()}
            onmouseenter={() => (activeIndex = index)}
            onclick={() => choose(entry)}
          >
            {#if entry.type === "command"}
              <span class="text-fg">{entry.command.label}</span>
            {:else}
              <span class="flex items-baseline justify-between gap-3">
                <span class="truncate text-fg">{entry.hit.title}</span>
                <span class="shrink-0 text-xs text-muted">{kindLabel(entry.hit.kind)}</span>
              </span>
              {#if entry.hit.snippet}
                <span class="truncate text-xs text-muted">
                  {#each parts(entry.hit.snippet) as part, i (i)}
                    {#if part.match}<mark class="bg-accent/25 text-fg">{part.text}</mark
                      >{:else}{part.text}{/if}
                  {/each}
                </span>
              {/if}
            {/if}
          </li>
        {/each}
      </ul>
      {#if query.trim() && entries.length === 0}
        <p class="px-3 py-2 text-sm text-muted">{m.ui_no_results()}</p>
      {/if}
    </div>
  </Dialog>
{/if}
