<script lang="ts">
  import { untrack } from "svelte";
  import CalendarPlus from "@lucide/svelte/icons/calendar-plus";
  import ListPlus from "@lucide/svelte/icons/list-plus";
  import Trash2 from "@lucide/svelte/icons/trash-2";
  import { resolve } from "$app/paths";
  import {
    commands,
    type Contact_Serialize as Contact,
    type FeedCursor,
    type InteractionKind,
    type Interaction_Serialize as Interaction,
    type Project_Serialize as Project,
  } from "$lib/api/bindings";
  import { call } from "$lib/api/client";
  import { errorMessage } from "$lib/i18n/errors";
  import { i18n } from "$lib/i18n/locale.svelte";
  import Markdown from "$lib/markdown/Markdown.svelte";
  import MarkdownEditor from "$lib/markdown/MarkdownEditor.svelte";
  import { m } from "$lib/paraglide/messages";
  import Button from "$lib/ui/Button.svelte";
  import DateTimePicker from "$lib/ui/DateTimePicker.svelte";
  import { confirm, prompt } from "$lib/ui/dialogs.svelte";
  import Select from "$lib/ui/Select.svelte";
  import Spinner from "$lib/ui/Spinner.svelte";
  import { toast } from "$lib/ui/toast.svelte";
  import EventDialog from "../calendar/EventDialog.svelte";
  import { localDateTime } from "../calendar/time";
  import { fromLocal } from "../time/format";

  let { clientId, now = () => Date.now() }: { clientId: string; now?: () => number } = $props();

  const PAGE = 20;
  const NONE = "";
  const KINDS: InteractionKind[] = ["call", "meeting", "email", "message", "other"];
  const kindLabel = (k: InteractionKind) =>
    ({
      call: m.interaction_kind_call,
      meeting: m.interaction_kind_meeting,
      email: m.interaction_kind_email,
      message: m.interaction_kind_message,
      other: m.interaction_kind_other,
    })[k]();

  let kind = $state<InteractionKind | null>(null);
  let items = $state<Interaction[] | null>(null);
  let next = $state<FeedCursor | null>(null);
  let loading = $state(false);
  let adding = $state(false);
  let contacts = $state<Contact[]>([]);
  let projects = $state<Project[]>([]);
  let draft = $state(blank());
  let saving = $state(false);
  /** The interaction an event is being planned from. */
  let planning = $state<Interaction | null>(null);
  let sentinel = $state<HTMLElement>();

  const showError = (err: unknown) => toast.show({ kind: "error", message: errorMessage(err) });

  function blank() {
    return {
      kind: "call" as InteractionKind,
      when: localDateTime(now()),
      contactId: null as string | null,
      projectId: null as string | null,
      summary: "",
    };
  }

  /** Loads the first page (`more` = false) or the next one. */
  async function load(more: boolean) {
    if (loading || (more && !next)) return;
    loading = true;
    try {
      const page = await call(commands.interactionFeed(clientId, kind, more ? next : null, PAGE));
      items = more ? [...(items ?? []), ...page.items] : page.items;
      next = page.next;
    } catch (err) {
      showError(err);
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    void [clientId, kind];
    untrack(() => {
      next = null;
      void load(false);
    });
  });

  // Loads more as the end of the list scrolls into view.
  $effect(() => {
    if (!sentinel) return;
    const observer = new IntersectionObserver((entries) => {
      if (entries.some((e) => e.isIntersecting)) void load(true);
    });
    observer.observe(sentinel);
    return () => observer.disconnect();
  });

  $effect(() => {
    if (!adding) return;
    void Promise.all([
      call(commands.listContacts(clientId)),
      call(commands.listProjects({ query: null, status: null, clientId, archived: false })),
    ])
      .then(([c, p]) => {
        contacts = c;
        projects = p;
      })
      .catch(showError);
  });

  async function save(event: SubmitEvent) {
    event.preventDefault();
    saving = true;
    try {
      await call(
        commands.createInteraction({
          clientId,
          contactId: draft.contactId,
          projectId: draft.projectId,
          kind: draft.kind,
          occurredAt: fromLocal(draft.when),
          summaryMd: draft.summary,
        }),
      );
      adding = false;
      draft = blank();
      await load(false);
    } catch (err) {
      showError(err);
    } finally {
      saving = false;
    }
  }

  async function followUpTask(item: Interaction) {
    const title = await prompt({
      title: m.interaction_create_task(),
      label: m.interaction_task_title(),
      confirmLabel: m.interaction_create(),
    });
    if (!title) return;
    try {
      await call(commands.followUpTask(item.id, title, null));
      await load(false);
    } catch (err) {
      showError(err);
    }
  }

  async function remove(item: Interaction) {
    const ok = await confirm({
      title: m.interaction_delete_title(),
      confirmLabel: m.ui_delete(),
      danger: true,
    });
    if (!ok) return;
    try {
      await call(commands.deleteInteraction(item.id));
      await load(false);
    } catch (err) {
      showError(err);
    }
  }

  const when = (ts: number) =>
    new Intl.DateTimeFormat(i18n.locale, { dateStyle: "medium", timeStyle: "short" }).format(ts);

  /** Tomorrow 10:00–11:00 local: a default slot for a follow-up event. */
  function tomorrow() {
    const date = localDateTime(now() + 86_400_000).slice(0, 10);
    return { start: `${date}T10:00`, end: `${date}T11:00`, allDay: false };
  }
</script>

<div class="flex flex-col gap-4">
  <div class="flex flex-wrap items-end gap-3">
    <div class="w-48">
      <Select
        label={m.interaction_type()}
        options={[
          { value: NONE, label: m.interaction_all_types() },
          ...KINDS.map((k) => ({ value: k, label: kindLabel(k) })),
        ]}
        bind:value={() => kind ?? NONE, (v) => (kind = (v || null) as InteractionKind | null)}
      />
    </div>
    <span class="flex-1"></span>
    {#if !adding}
      <Button onclick={() => (adding = true)}>{m.interaction_add()}</Button>
    {/if}
  </div>

  {#if adding}
    <form
      aria-label={m.interaction_new()}
      class="flex flex-col gap-3 rounded-[var(--radius)] border border-border p-4"
      onsubmit={save}
    >
      <div class="grid gap-3 sm:grid-cols-2">
        <Select
          label={m.interaction_type()}
          options={KINDS.map((k) => ({ value: k, label: kindLabel(k) }))}
          bind:value={() => draft.kind, (v) => (draft.kind = (v ?? "call") as InteractionKind)}
        />
        <DateTimePicker
          dateLabel={m.interaction_date()}
          timeLabel={m.interaction_time()}
          bind:value={() => draft.when, (v) => (draft.when = v ?? draft.when)}
        />
        <Select
          label={m.interaction_contact()}
          options={[
            { value: NONE, label: m.interaction_no_contact() },
            ...contacts.map((c) => ({ value: c.id, label: c.displayName })),
          ]}
          bind:value={() => draft.contactId ?? NONE, (v) => (draft.contactId = v || null)}
        />
        <Select
          label={m.interaction_project()}
          options={[
            { value: NONE, label: m.interaction_no_project() },
            ...projects.map((p) => ({ value: p.id, label: p.name })),
          ]}
          bind:value={() => draft.projectId ?? NONE, (v) => (draft.projectId = v || null)}
        />
      </div>
      <MarkdownEditor label={m.interaction_summary()} rows={3} bind:value={draft.summary} />
      <div class="flex gap-2">
        <Button
          type="submit"
          variant="primary"
          loading={saving}
          disabled={draft.summary.trim() === ""}
        >
          {m.ui_save()}
        </Button>
        <Button onclick={() => (adding = false)}>{m.ui_cancel()}</Button>
      </div>
    </form>
  {/if}

  {#if items === null}
    <div class="flex justify-center p-4"><Spinner /></div>
  {:else if items.length === 0}
    <p class="text-sm text-muted">{m.interaction_empty()}</p>
  {:else}
    <div
      role="feed"
      aria-label={m.client_tab_history()}
      aria-busy={loading}
      class="flex flex-col gap-3"
    >
      {#each items as item, i (item.id)}
        <article
          aria-posinset={i + 1}
          aria-setsize={next ? -1 : items.length}
          aria-labelledby="interaction-{item.id}"
          class="flex flex-col gap-2 rounded-[var(--radius)] border border-border p-3"
        >
          <header class="flex flex-wrap items-center gap-2 text-sm">
            <span id="interaction-{item.id}" class="font-medium text-fg"
              >{kindLabel(item.kind)}</span
            >
            <span class="text-muted">{when(item.occurredAt)}</span>
            {#if item.contactName}<span class="text-muted">· {item.contactName}</span>{/if}
            {#if item.projectName}<span class="text-muted">· {item.projectName}</span>{/if}
            <span class="flex-1"></span>
            <Button
              variant="ghost"
              size="sm"
              aria-label={m.interaction_create_task()}
              onclick={() => followUpTask(item)}
            >
              <ListPlus class="size-4" aria-hidden="true" />
            </Button>
            <Button
              variant="ghost"
              size="sm"
              aria-label={m.interaction_create_event()}
              onclick={() => (planning = item)}
            >
              <CalendarPlus class="size-4" aria-hidden="true" />
            </Button>
            <Button
              variant="ghost"
              size="sm"
              aria-label={m.interaction_delete()}
              onclick={() => remove(item)}
            >
              <Trash2 class="size-4" aria-hidden="true" />
            </Button>
          </header>
          <Markdown source={item.summaryMd} />
          {#if item.followUps.length > 0}
            <ul aria-label={m.interaction_follow_ups()} class="flex flex-wrap gap-2 text-sm">
              {#each item.followUps as f (f.id)}
                <li>
                  {#if f.kind === "task"}
                    <a
                      class="text-accent hover:underline"
                      href={resolve("/tasks/[id]", { id: f.id })}>{f.title}</a
                    >
                  {:else}
                    <a class="text-accent hover:underline" href={resolve("/calendar")}>{f.title}</a>
                  {/if}
                </li>
              {/each}
            </ul>
          {/if}
        </article>
      {/each}
    </div>
    {#if next}
      <div bind:this={sentinel} class="flex justify-center">
        <Button variant="ghost" {loading} onclick={() => load(true)}
          >{m.interaction_load_more()}</Button
        >
      </div>
    {/if}
  {/if}
</div>

{#if planning}
  {@const source = planning}
  <EventDialog
    initial={tomorrow()}
    create={(input) => call(commands.followUpEvent(source.id, input))}
    onsaved={() => {
      planning = null;
      void load(false);
    }}
    onclose={() => (planning = null)}
  />
{/if}
