<script lang="ts">
  import Ellipsis from "@lucide/svelte/icons/ellipsis";
  import Pencil from "@lucide/svelte/icons/pencil";
  import { goto } from "$app/navigation";
  import { resolve } from "$app/paths";
  import { commands, type EntityRef, type Project_Serialize as Project } from "$lib/api/bindings";
  import { call } from "$lib/api/client";
  import { errorMessage } from "$lib/i18n/errors";
  import { formatIsoDate, formatMoney } from "$lib/i18n/format";
  import { i18n } from "$lib/i18n/locale.svelte";
  import Markdown from "$lib/markdown/Markdown.svelte";
  import { m } from "$lib/paraglide/messages";
  import Button from "$lib/ui/Button.svelte";
  import { confirm } from "$lib/ui/dialogs.svelte";
  import DropdownMenu from "$lib/ui/DropdownMenu.svelte";
  import type { MenuEntry } from "$lib/ui/menu";
  import Spinner from "$lib/ui/Spinner.svelte";
  import Tabs from "$lib/ui/Tabs.svelte";
  import { toast } from "$lib/ui/toast.svelte";
  import FieldsPanel from "../fields/FieldsPanel.svelte";
  import NotesPanel from "../notes/NotesPanel.svelte";
  import { billingLabel, projectStatusLabel } from "./labels";

  let { id }: { id: string } = $props();

  let project = $state<Project | null>(null);
  let loadError = $state<string | null>(null);
  let tab = $state("overview");
  let busy = $state(false);

  let owner = $derived<EntityRef>({ type: "project", id });
  // Sections of later milestones are shown as placeholders for now.
  let tabs = $derived([
    { id: "overview", label: m.client_tab_overview() },
    { id: "tasks", label: m.client_tab_tasks() },
    { id: "time", label: m.client_tab_time() },
    { id: "secrets", label: m.client_tab_secrets() },
    { id: "files", label: m.client_tab_files() },
    { id: "notes", label: m.client_tab_notes() },
    { id: "fields", label: m.client_tab_fields() },
    { id: "finance", label: m.client_tab_finance() },
  ]);
  let actions = $derived<MenuEntry[]>([
    project?.archivedAt === null
      ? { id: "archive", label: m.client_archive() }
      : { id: "restore", label: m.client_restore() },
    { separator: true },
    { id: "delete", label: m.project_delete(), danger: true },
  ]);

  $effect(() => {
    const projectId = id;
    loadError = null;
    call(commands.getProject(projectId)).then(
      (p) => (project = p),
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

  async function onAction(action: string) {
    if (action === "archive") {
      await act(async () => {
        project = await call(commands.archiveProject(id));
        toast.show({ message: m.project_archived_toast() });
      });
    } else if (action === "restore") {
      await act(async () => {
        project = await call(commands.restoreProject(id));
        toast.show({ message: m.project_restored_toast() });
      });
    } else if (action === "delete" && project) {
      const ok = await confirm({
        title: m.project_delete_title({ name: project.name }),
        message: m.project_delete_message(),
        confirmLabel: m.ui_delete(),
        danger: true,
      });
      if (!ok) return;
      await act(async () => {
        await call(commands.deleteProject(id));
        toast.show({ message: m.project_deleted_toast() });
        await goto(resolve("/projects"));
      });
    }
  }

  function price(p: Project): string {
    if (p.billingType === "fixed") {
      return p.budgetMinor === null
        ? m.client_not_set()
        : formatMoney(p.budgetMinor, p.currency, i18n.locale);
    }
    if (p.rateMinor === null) return m.client_not_set();
    return m.client_rate_per_hour({ amount: formatMoney(p.rateMinor, p.currency, i18n.locale) });
  }

  const date = (iso: string | null) => (iso ? formatIsoDate(iso, i18n.locale) : m.client_not_set());
</script>

{#if loadError}
  <p class="text-sm text-danger" role="alert">{loadError}</p>
{:else if !project}
  <div class="flex justify-center p-8"><Spinner /></div>
{:else}
  <div class="flex flex-col gap-6">
    <header class="flex items-start gap-3">
      <div class="min-w-0 flex-1">
        <h1 class="truncate text-xl font-semibold text-fg">{project.name}</h1>
        <p class="flex gap-2 text-sm text-muted">
          <a
            href={resolve("/clients/[id]", { id: project.clientId })}
            class="hover:underline focus-visible:outline-2 focus-visible:outline-focus"
          >
            {project.clientName}
          </a>
          {#if project.archivedAt !== null}
            <span aria-hidden="true">·</span>
            <span class="text-danger">{m.client_archived()}</span>
          {/if}
        </p>
      </div>
      <Button onclick={() => goto(resolve("/projects/[id]/edit", { id }))}>
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
    </header>

    <Tabs label={m.project_tabs()} {tabs} bind:value={tab}>
      {#snippet panel(current)}
        <div class="pt-4">
          {#if current === "overview"}
            {@const p = project!}
            <div class="flex max-w-3xl flex-col gap-6">
              <dl class="grid grid-cols-[12rem_1fr] gap-x-4 gap-y-3 text-sm">
                <dt class="text-muted">{m.project_field_status()}</dt>
                <dd class="text-fg">{projectStatusLabel(p.status)}</dd>
                <dt class="text-muted">{m.project_field_billing()}</dt>
                <dd class="text-fg">{billingLabel(p.billingType)}</dd>
                <dt class="text-muted">
                  {p.billingType === "fixed" ? m.project_field_budget() : m.project_field_rate()}
                </dt>
                <dd class="text-fg tabular-nums">{price(p)}</dd>
                <dt class="text-muted">{m.project_field_start()}</dt>
                <dd class="text-fg">{date(p.startDate)}</dd>
                <dt class="text-muted">{m.project_field_deadline()}</dt>
                <dd class="text-fg">{date(p.deadline)}</dd>
              </dl>
              <section class="flex flex-col gap-2">
                <h2 class="text-base font-semibold text-fg">{m.project_field_description()}</h2>
                {#if p.descriptionMd.trim()}
                  <Markdown source={p.descriptionMd} />
                {:else}
                  <p class="text-sm text-muted">{m.project_no_description()}</p>
                {/if}
              </section>
            </div>
          {:else if current === "notes"}
            <NotesPanel {owner} />
          {:else if current === "fields"}
            <FieldsPanel {owner} />
          {:else}
            <p class="text-sm text-muted">{m.page_coming_soon()}</p>
          {/if}
        </div>
      {/snippet}
    </Tabs>
  </div>
{/if}
