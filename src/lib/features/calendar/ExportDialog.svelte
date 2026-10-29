<script lang="ts">
  import { save } from "@tauri-apps/plugin-dialog";
  import {
    commands,
    type Client_Serialize as Client,
    type ExportFilter,
    type Project_Serialize as Project,
  } from "$lib/api/bindings";
  import { call } from "$lib/api/client";
  import { errorMessage } from "$lib/i18n/errors";
  import { m } from "$lib/paraglide/messages";
  import Button from "$lib/ui/Button.svelte";
  import { addDays } from "$lib/ui/date";
  import DatePicker from "$lib/ui/DatePicker.svelte";
  import Dialog from "$lib/ui/Dialog.svelte";
  import RadioGroup from "$lib/ui/RadioGroup.svelte";
  import Select from "$lib/ui/Select.svelte";
  import { toast } from "$lib/ui/toast.svelte";
  import { midnight } from "./time";

  let { onclose }: { onclose: () => void } = $props();

  type Scope = "all" | "client" | "project" | "range";
  let scope = $state<Scope>("all");
  let clientId = $state<string | null>(null);
  let projectId = $state<string | null>(null);
  let from = $state<string | null>(null);
  let to = $state<string | null>(null);
  let clients = $state<Client[]>([]);
  let projects = $state<Project[]>([]);
  let busy = $state(false);

  const showError = (err: unknown) => toast.show({ kind: "error", message: errorMessage(err) });

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

  const scopes = [
    { value: "all", label: m.ics_export_all() },
    { value: "client", label: m.ics_export_client() },
    { value: "project", label: m.ics_export_project() },
    { value: "range", label: m.ics_export_range() },
  ];

  function filter(): ExportFilter {
    return {
      clientId: scope === "client" ? clientId : null,
      projectId: scope === "project" ? projectId : null,
      // The "to" date is the last day included.
      from: scope === "range" && from ? midnight(from) : null,
      to: scope === "range" && to ? midnight(addDays(to, 1)) : null,
    };
  }

  async function exportFile() {
    const path = await save({
      defaultPath: "calendar.ics",
      filters: [{ name: "iCalendar", extensions: ["ics"] }],
    }).catch(() => null);
    if (!path) return;
    busy = true;
    try {
      await call(commands.exportIcs(path, filter()));
      toast.show({ message: m.ics_exported_toast() });
      onclose();
    } catch (err) {
      showError(err);
    } finally {
      busy = false;
    }
  }
</script>

<Dialog open title={m.ics_export_title()} {onclose}>
  <div class="flex w-[28rem] max-w-full flex-col gap-4">
    <RadioGroup
      label={m.ics_export_scope()}
      options={scopes}
      bind:value={() => scope, (v) => (scope = v as Scope)}
    />
    {#if scope === "client"}
      <Select
        label={m.event_client()}
        options={clients.map((c) => ({ value: c.id, label: c.displayName }))}
        bind:value={clientId}
      />
    {:else if scope === "project"}
      <Select
        label={m.event_project()}
        options={projects.map((p) => ({ value: p.id, label: `${p.name} · ${p.clientName}` }))}
        bind:value={projectId}
      />
    {:else if scope === "range"}
      <div class="grid gap-4 sm:grid-cols-2">
        <DatePicker label={m.ics_export_from()} bind:value={from} />
        <DatePicker label={m.ics_export_to()} bind:value={to} />
      </div>
    {/if}
  </div>
  {#snippet footer()}
    <Button onclick={onclose}>{m.ui_cancel()}</Button>
    <Button
      variant="primary"
      loading={busy}
      disabled={(scope === "client" && !clientId) || (scope === "project" && !projectId)}
      onclick={exportFile}
    >
      {m.ics_export_submit()}
    </Button>
  {/snippet}
</Dialog>
