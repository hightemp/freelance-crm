<script lang="ts">
  import { untrack } from "svelte";
  import Pencil from "@lucide/svelte/icons/pencil";
  import Pin from "@lucide/svelte/icons/pin";
  import PinOff from "@lucide/svelte/icons/pin-off";
  import Trash2 from "@lucide/svelte/icons/trash-2";
  import { commands, type EntityRef, type Note } from "$lib/api/bindings";
  import { call } from "$lib/api/client";
  import { errorMessage } from "$lib/i18n/errors";
  import { formatDate } from "$lib/i18n/format";
  import Markdown from "$lib/markdown/Markdown.svelte";
  import MarkdownEditor from "$lib/markdown/MarkdownEditor.svelte";
  import { i18n } from "$lib/i18n/locale.svelte";
  import { m } from "$lib/paraglide/messages";
  import Button from "$lib/ui/Button.svelte";
  import { confirm } from "$lib/ui/dialogs.svelte";
  import Spinner from "$lib/ui/Spinner.svelte";
  import { toast } from "$lib/ui/toast.svelte";

  let { owner }: { owner: EntityRef } = $props();

  let notes = $state<Note[] | null>(null);
  let draft = $state("");
  let editing = $state<{ id: string; body: string } | null>(null);
  let busy = $state(false);

  async function run(action: () => Promise<unknown>) {
    busy = true;
    try {
      await action();
      notes = await call(commands.listNotes(owner));
    } catch (err) {
      toast.show({ kind: "error", message: errorMessage(err) });
    } finally {
      busy = false;
    }
  }

  // Reload when the owner changes (it is read after an await in `run`).
  $effect(() => {
    void [owner.type, owner.id];
    untrack(() => run(async () => {}));
  });

  function add(event: SubmitEvent) {
    event.preventDefault();
    const bodyMd = draft;
    void run(async () => {
      await call(commands.createNote(owner, { bodyMd, pinned: false }));
      draft = "";
    });
  }

  function save(note: Note) {
    const bodyMd = editing!.body;
    void run(async () => {
      await call(commands.updateNote(note.id, { bodyMd, pinned: note.pinned }));
      editing = null;
    });
  }

  async function remove(note: Note) {
    const ok = await confirm({
      title: m.note_delete_title(),
      confirmLabel: m.ui_delete(),
      danger: true,
    });
    if (ok) void run(() => call(commands.deleteNote(note.id)));
  }
</script>

<div class="flex flex-col gap-4">
  <form class="flex flex-col gap-2" onsubmit={add}>
    <MarkdownEditor label={m.notes_new()} rows={3} bind:value={draft} />
    <div>
      <Button type="submit" variant="primary" disabled={busy || draft.trim() === ""}>
        {m.notes_add()}
      </Button>
    </div>
  </form>

  {#if notes === null}
    <div class="flex justify-center p-6"><Spinner /></div>
  {:else if notes.length === 0}
    <p class="text-sm text-muted">{m.notes_empty()}</p>
  {:else}
    {#each notes as note (note.id)}
      <article class="flex flex-col gap-2 rounded-[var(--radius)] border border-border p-3">
        <header class="flex items-center gap-2 text-xs text-muted">
          {#if note.pinned}
            <span class="font-medium text-accent">{m.note_pinned()}</span>
          {/if}
          <span>{formatDate(note.updatedAt, i18n.locale, { withTime: true })}</span>
          <span class="ml-auto flex gap-1">
            <Button
              variant="ghost"
              size="sm"
              aria-label={note.pinned ? m.note_unpin() : m.note_pin()}
              aria-pressed={note.pinned}
              disabled={busy}
              onclick={() => run(() => call(commands.setNotePinned(note.id, !note.pinned)))}
            >
              {#if note.pinned}
                <PinOff class="size-4" aria-hidden="true" />
              {:else}
                <Pin class="size-4" aria-hidden="true" />
              {/if}
            </Button>
            <Button
              variant="ghost"
              size="sm"
              aria-label={m.ui_edit()}
              onclick={() => (editing = { id: note.id, body: note.bodyMd })}
            >
              <Pencil class="size-4" aria-hidden="true" />
            </Button>
            <Button
              variant="ghost"
              size="sm"
              aria-label={m.ui_delete()}
              onclick={() => remove(note)}
            >
              <Trash2 class="size-4" aria-hidden="true" />
            </Button>
          </span>
        </header>
        {#if editing?.id === note.id}
          <MarkdownEditor label={m.note_edit()} rows={4} bind:value={editing.body} />
          <div class="flex gap-2">
            <Button
              variant="primary"
              disabled={busy || editing.body.trim() === ""}
              onclick={() => save(note)}
            >
              {m.ui_save()}
            </Button>
            <Button onclick={() => (editing = null)}>{m.ui_cancel()}</Button>
          </div>
        {:else}
          <div data-testid="note-body">
            <Markdown source={note.bodyMd} />
          </div>
        {/if}
      </article>
    {/each}
  {/if}
</div>
