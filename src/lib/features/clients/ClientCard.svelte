<script lang="ts">
  import Ellipsis from "@lucide/svelte/icons/ellipsis";
  import Pencil from "@lucide/svelte/icons/pencil";
  import Plus from "@lucide/svelte/icons/plus";
  import { goto } from "$app/navigation";
  import { resolve } from "$app/paths";
  import {
    commands,
    type ContactMethod_Serialize as ContactMethod,
    type Client_Serialize as Client,
    type DeleteImpact,
    type EntityRef,
  } from "$lib/api/bindings";
  import { call } from "$lib/api/client";
  import { gotoWithQuery } from "$lib/app/navigate";
  import { appSettings } from "$lib/app/settings.svelte";
  import { errorMessage } from "$lib/i18n/errors";
  import { formatDate, formatMoney } from "$lib/i18n/format";
  import { i18n } from "$lib/i18n/locale.svelte";
  import { m } from "$lib/paraglide/messages";
  import Button from "$lib/ui/Button.svelte";
  import DropdownMenu from "$lib/ui/DropdownMenu.svelte";
  import type { MenuEntry } from "$lib/ui/menu";
  import Spinner from "$lib/ui/Spinner.svelte";
  import Tabs from "$lib/ui/Tabs.svelte";
  import { toast } from "$lib/ui/toast.svelte";
  import { methodLabel } from "../contact-methods/methods";
  import ContactsPanel from "../contacts/ContactsPanel.svelte";
  import FieldsPanel from "../fields/FieldsPanel.svelte";
  import AttachmentPanel from "../files/AttachmentPanel.svelte";
  import NotesPanel from "../notes/NotesPanel.svelte";
  import ProjectList from "../projects/ProjectList.svelte";
  import SecretList from "../secrets/SecretList.svelte";
  import TaskList from "../tasks/TaskList.svelte";
  import VaultScreen from "../vault/VaultScreen.svelte";
  import RequisitesForm from "../requisites/RequisitesForm.svelte";
  import TagEditor from "../tags/TagEditor.svelte";
  import ClientDeleteDialog from "./ClientDeleteDialog.svelte";
  import { statusLabel } from "./status";

  let { id }: { id: string } = $props();

  let client = $state<Client | null>(null);
  let methods = $state<ContactMethod[]>([]);
  let loadError = $state<string | null>(null);
  let tab = $state("overview");
  /** Set while the delete confirmation is open. */
  let deleting = $state<DeleteImpact | null>(null);
  let busy = $state(false);

  let actions = $derived<MenuEntry[]>([
    client?.archivedAt === null
      ? { id: "archive", label: m.client_archive() }
      : { id: "restore", label: m.client_restore() },
    { separator: true },
    { id: "delete", label: m.client_delete(), danger: true },
  ]);

  let owner = $derived<EntityRef>({ type: "client", id });

  // Sections of later milestones are shown as placeholders for now.
  let tabs = $derived([
    { id: "overview", label: m.client_tab_overview() },
    ...(client?.kind === "company" ? [{ id: "contacts", label: m.client_tab_contacts() }] : []),
    { id: "requisites", label: m.client_tab_requisites() },
    { id: "fields", label: m.client_tab_fields() },
    { id: "notes", label: m.client_tab_notes() },
    { id: "projects", label: m.client_tab_projects() },
    { id: "tasks", label: m.client_tab_tasks() },
    { id: "events", label: m.client_tab_events() },
    { id: "secrets", label: m.client_tab_secrets() },
    { id: "files", label: m.client_tab_files() },
    { id: "time", label: m.client_tab_time() },
    { id: "finance", label: m.client_tab_finance() },
    { id: "history", label: m.client_tab_history() },
  ]);

  $effect(() => {
    const clientId = id;
    loadError = null;
    Promise.all([
      call(commands.getClient(clientId)),
      call(commands.listContactMethods({ type: "client", id: clientId })),
    ]).then(
      ([c, ms]) => {
        client = c;
        methods = ms;
      },
      (err) => (loadError = errorMessage(err)),
    );
  });

  async function act(action: () => Promise<void>) {
    busy = true;
    try {
      await action();
    } catch (err) {
      toast.show({ kind: "error", message: errorMessage(err) });
    } finally {
      busy = false;
    }
  }

  const archive = () =>
    act(async () => {
      client = await call(commands.archiveClient(id));
      deleting = null;
      toast.show({ message: m.client_archived_toast() });
    });

  const restore = () =>
    act(async () => {
      client = await call(commands.restoreClient(id));
      toast.show({ message: m.client_restored_toast() });
    });

  const askDelete = () =>
    act(async () => {
      deleting = await call(commands.clientDeleteImpact(id));
    });

  const remove = () =>
    act(async () => {
      await call(commands.deleteClient(id));
      deleting = null;
      toast.show({ message: m.client_deleted_toast() });
      await goto(resolve("/clients"));
    });

  function onAction(action: string) {
    if (action === "archive") void archive();
    else if (action === "restore") void restore();
    else if (action === "delete") void askDelete();
  }

  const newProject = () => gotoWithQuery(resolve("/projects/new"), { client: id });
  const newTask = () => gotoWithQuery(resolve("/tasks/new"), { client: id });

  function rate(c: Client): string {
    if (c.defaultRateMinor === null) return m.client_not_set();
    const currency = c.defaultCurrency ?? appSettings.value.defaultCurrency;
    return m.client_rate_per_hour({
      amount: formatMoney(c.defaultRateMinor, currency, i18n.locale),
    });
  }
</script>

{#if loadError}
  <p class="text-sm text-danger" role="alert">{loadError}</p>
{:else if !client}
  <div class="flex justify-center p-8"><Spinner /></div>
{:else}
  <div class="flex flex-col gap-6">
    <header class="flex flex-col gap-3">
      <div class="flex items-start gap-3">
        <div class="min-w-0 flex-1">
          <h1 class="truncate text-xl font-semibold text-fg">{client.displayName}</h1>
          <p class="flex gap-2 text-sm text-muted">
            <span
              >{client.kind === "person" ? m.client_kind_person() : m.client_kind_company()}</span
            >
            <span aria-hidden="true">·</span>
            <span>{statusLabel(client.status)}</span>
            {#if client.archivedAt !== null}
              <span aria-hidden="true">·</span>
              <span class="text-danger">{m.client_archived()}</span>
            {/if}
          </p>
        </div>
        <Button onclick={() => goto(resolve("/clients/[id]/edit", { id }))}>
          <Pencil class="size-4" aria-hidden="true" />
          {m.ui_edit()}
        </Button>
        <DropdownMenu items={actions} onSelect={onAction} placement="bottom-end">
          {#snippet trigger(props)}
            <Button {...props} aria-label={m.client_actions()} disabled={busy}>
              <Ellipsis class="size-4" aria-hidden="true" />
            </Button>
          {/snippet}
        </DropdownMenu>
      </div>
      <div class="max-w-xl">
        <TagEditor {owner} label={m.client_tags()} bind:tagIds={client.tagIds} />
      </div>
    </header>

    <Tabs label={m.client_tabs()} {tabs} bind:value={tab}>
      {#snippet panel(current)}
        <div class="pt-4">
          {#if current === "overview"}
            <dl class="grid max-w-2xl grid-cols-[12rem_1fr] gap-x-4 gap-y-3 text-sm">
              {#if client!.legalName}
                <dt class="text-muted">{m.client_field_legal_name()}</dt>
                <dd class="text-fg">{client!.legalName}</dd>
              {/if}
              <dt class="text-muted">{m.contact_methods()}</dt>
              <dd class="flex flex-col gap-1 text-fg">
                {#each methods as method (method.id)}
                  <span>
                    <span class="text-muted">{method.label ?? methodLabel(method.kind)}:</span>
                    {method.value}
                  </span>
                {:else}
                  <span class="text-muted">{m.client_no_methods()}</span>
                {/each}
              </dd>
              <dt class="text-muted">{m.client_field_rate()}</dt>
              <dd class="text-fg tabular-nums">{rate(client!)}</dd>
              <dt class="text-muted">{m.client_created()}</dt>
              <dd class="text-fg">{formatDate(client!.createdAt, i18n.locale)}</dd>
              <dt class="text-muted">{m.client_updated()}</dt>
              <dd class="text-fg">{formatDate(client!.updatedAt, i18n.locale)}</dd>
            </dl>
          {:else if current === "contacts"}
            <ContactsPanel clientId={id} />
          {:else if current === "requisites"}
            <RequisitesForm owner={{ type: "client", id }} />
          {:else if current === "projects"}
            <div class="flex flex-col gap-4">
              <div>
                <Button onclick={newProject}>
                  <Plus class="size-4" aria-hidden="true" />
                  {m.projects_new()}
                </Button>
              </div>
              <ProjectList clientId={id} />
            </div>
          {:else if current === "tasks"}
            <div class="flex h-[32rem] flex-col gap-4">
              <div>
                <Button onclick={newTask}>
                  <Plus class="size-4" aria-hidden="true" />
                  {m.tasks_new()}
                </Button>
              </div>
              <div class="min-h-0 flex-1">
                <TaskList scope={{ type: "client", id }} />
              </div>
            </div>
          {:else if current === "secrets"}
            <VaultScreen>
              <div>
                <Button onclick={() => gotoWithQuery(resolve("/vault/new"), { client: id })}>
                  <Plus class="size-4" aria-hidden="true" />
                  {m.secrets_new()}
                </Button>
              </div>
              <SecretList clientId={id} />
            </VaultScreen>
          {:else if current === "fields"}
            <FieldsPanel {owner} />
          {:else if current === "files"}
            <AttachmentPanel {owner} />
          {:else if current === "notes"}
            <NotesPanel {owner} />
          {:else}
            <p class="text-sm text-muted">{m.page_coming_soon()}</p>
          {/if}
        </div>
      {/snippet}
    </Tabs>
  </div>
{/if}

{#if client && deleting}
  <ClientDeleteDialog
    name={client.displayName}
    impact={deleting}
    archived={client.archivedAt !== null}
    {busy}
    ondelete={remove}
    onarchive={archive}
    oncancel={() => (deleting = null)}
  />
{/if}
