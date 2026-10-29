<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import {
    commands,
    type Client_Serialize as Client,
    type IcsEvent_Serialize as IcsEvent,
    type ParsedIcs_Serialize as ParsedIcs,
    type Project_Serialize as Project,
  } from "$lib/api/bindings";
  import { call } from "$lib/api/client";
  import { codeMessage, errorMessage } from "$lib/i18n/errors";
  import { m } from "$lib/paraglide/messages";
  import Button from "$lib/ui/Button.svelte";
  import Checkbox from "$lib/ui/Checkbox.svelte";
  import Dialog from "$lib/ui/Dialog.svelte";
  import Select from "$lib/ui/Select.svelte";
  import { toast } from "$lib/ui/toast.svelte";

  let { onclose }: { onclose: () => void } = $props();

  const NONE = "";
  let path = $state<string | null>(null);
  let parsed = $state<ParsedIcs | null>(null);
  /** UIDs to import. */
  let selected = $state<string[]>([]);
  let clientId = $state<string | null>(null);
  let projectId = $state<string | null>(null);
  let clients = $state<Client[]>([]);
  let projects = $state<Project[]>([]);
  let busy = $state(false);

  const showError = (err: unknown) => toast.show({ kind: "error", message: errorMessage(err) });

  // One row per event: edited occurrences go with their series.
  let rows = $derived(
    (parsed?.events ?? []).filter(
      (e: IcsEvent, i: number, all: IcsEvent[]) =>
        e.recurrenceId === null ||
        !all.some((other) => other.uid === e.uid && other.recurrenceId === null),
    ),
  );
  let uids = $derived([...new Set(rows.map((e) => e.uid))]);

  $effect(() => {
    void Promise.all([
      call(commands.listClients({ query: null, status: null, archived: false, tagIds: [] })),
      call(commands.listProjects({ query: null, status: null, clientId: null, archived: false })),
    ])
      .then(([c, p]) => {
        clients = c;
        projects = p;
      })
      .catch(showError);
  });

  async function pick() {
    const chosen = await open({ filters: [{ name: "iCalendar", extensions: ["ics"] }] }).catch(
      () => null,
    );
    if (!chosen) return;
    busy = true;
    try {
      parsed = await call(commands.previewIcs(chosen));
      path = chosen;
      selected = [...new Set(parsed.events.map((e) => e.uid))];
    } catch (err) {
      showError(err);
    } finally {
      busy = false;
    }
  }

  function toggle(uid: string, on: boolean) {
    selected = on ? [...selected, uid] : selected.filter((u) => u !== uid);
  }

  async function importSelected() {
    if (!path) return;
    busy = true;
    try {
      const all = uids.every((u) => selected.includes(u));
      const report = await call(
        commands.importIcs(path, all ? null : uids.filter((u) => selected.includes(u)), {
          clientId,
          projectId,
        }),
      );
      toast.show({
        message: m.ics_imported_toast({ created: report.created, updated: report.updated }),
      });
      onclose();
    } catch (err) {
      showError(err);
    } finally {
      busy = false;
    }
  }

  let clientOptions = $derived([
    { value: NONE, label: m.event_no_client() },
    ...clients.map((c) => ({ value: c.id, label: c.displayName })),
  ]);
  let projectOptions = $derived([
    { value: NONE, label: m.event_no_project() },
    ...projects
      .filter((p) => !clientId || p.clientId === clientId)
      .map((p) => ({ value: p.id, label: p.name })),
  ]);
  let count = $derived(uids.filter((u) => selected.includes(u)).length);
</script>

<Dialog open title={m.ics_import_title()} {onclose}>
  <div class="flex w-[36rem] max-w-full flex-col gap-4">
    <div>
      <Button onclick={pick} loading={busy && !parsed}>{m.ics_pick()}</Button>
      {#if path}<span class="ml-2 text-sm text-muted">{path.split(/[\\/]/).pop()}</span>{/if}
    </div>
    {#if parsed}
      <section class="flex flex-col gap-2">
        <h3 class="text-sm font-semibold text-fg">{m.ics_events()}</h3>
        <ul class="flex max-h-64 flex-col gap-1 overflow-y-auto">
          {#each rows as row (row.uid + (row.recurrenceId ?? ""))}
            <li class="flex items-center gap-2 text-sm">
              <Checkbox
                label={`${row.title || "—"} · ${row.start.replace("T", " ")}`}
                bind:checked={() => selected.includes(row.uid), (on) => toggle(row.uid, on)}
              />
              {#if row.rrule}<span class="rounded bg-surface px-1.5 text-xs text-muted"
                  >{m.ics_recurring()}</span
                >{/if}
            </li>
          {/each}
        </ul>
      </section>
      {#if parsed.issues.length > 0}
        <section class="flex flex-col gap-1">
          <h3 class="text-sm font-semibold text-fg">{m.ics_issues()}</h3>
          <ul class="flex flex-col gap-1 text-sm">
            {#each parsed.issues as issue, i (i)}
              <li class={issue.warning ? "text-muted" : "text-danger"}>
                {#if issue.line > 0}{m.ics_issue_line({ line: issue.line })}:
                {/if}{codeMessage(issue.error.code, issue.error.params)}
                {#if issue.uid}<span class="text-muted">({issue.uid})</span>{/if}
              </li>
            {/each}
          </ul>
        </section>
      {/if}
      <div class="grid gap-4 sm:grid-cols-2">
        <Select
          label={m.ics_link_client()}
          options={clientOptions}
          bind:value={
            () => clientId ?? NONE,
            (v) => {
              clientId = v || null;
              if (projectId && !projects.some((p) => p.id === projectId && p.clientId === clientId))
                projectId = null;
            }
          }
        />
        <Select
          label={m.ics_link_project()}
          options={projectOptions}
          bind:value={
            () => projectId ?? NONE,
            (v) => {
              projectId = v || null;
              const linked = projects.find((p) => p.id === projectId);
              if (linked) clientId = linked.clientId;
            }
          }
        />
      </div>
    {/if}
  </div>
  {#snippet footer()}
    <Button onclick={onclose}>{m.ui_cancel()}</Button>
    <Button
      variant="primary"
      disabled={!parsed || count === 0}
      loading={busy && !!parsed}
      onclick={importSelected}
    >
      {m.ics_import_submit({ count })}
    </Button>
  {/snippet}
</Dialog>
