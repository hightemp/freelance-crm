<script lang="ts">
  import { untrack } from "svelte";
  import Pencil from "@lucide/svelte/icons/pencil";
  import Plus from "@lucide/svelte/icons/plus";
  import Star from "@lucide/svelte/icons/star";
  import Trash2 from "@lucide/svelte/icons/trash-2";
  import {
    commands,
    type ContactInput_Deserialize as ContactInput,
    type ContactMethod_Serialize as ContactMethod,
    type Contact_Serialize as Contact,
  } from "$lib/api/bindings";
  import { AppError, call } from "$lib/api/client";
  import { codeMessage, errorMessage } from "$lib/i18n/errors";
  import { m } from "$lib/paraglide/messages";
  import Button from "$lib/ui/Button.svelte";
  import Checkbox from "$lib/ui/Checkbox.svelte";
  import Dialog from "$lib/ui/Dialog.svelte";
  import { confirm } from "$lib/ui/dialogs.svelte";
  import Spinner from "$lib/ui/Spinner.svelte";
  import TextArea from "$lib/ui/TextArea.svelte";
  import TextField from "$lib/ui/TextField.svelte";
  import { toast } from "$lib/ui/toast.svelte";
  import ContactMethodsEditor from "../contact-methods/ContactMethodsEditor.svelte";
  import {
    fromMethods,
    methodError,
    methodLabel,
    toMethodInputs,
    type MethodDraft,
  } from "../contact-methods/methods";

  let { clientId }: { clientId: string } = $props();

  interface Draft {
    id: string | null;
    lastName: string;
    firstName: string;
    middleName: string;
    position: string;
    note: string;
    isPrimary: boolean;
    methods: MethodDraft[];
  }

  const formId = $props.id();

  let contacts = $state<{ contact: Contact; methods: ContactMethod[] }[] | null>(null);
  let editing = $state<Draft | null>(null);
  let errors = $state<Record<string, string>>({});
  let busy = $state(false);

  let methodErrors = $derived(
    Object.fromEntries(
      Object.entries(errors).flatMap(([path, code]) => {
        const match = /^methods\.(\d+)$/.exec(path);
        return match ? [[Number(match[1]), code]] : [];
      }),
    ),
  );

  async function load() {
    const list = await call(commands.listContacts(clientId));
    contacts = await Promise.all(
      list.map(async (contact) => ({
        contact,
        methods: await call(commands.listContactMethods({ type: "contact", id: contact.id })),
      })),
    );
  }

  async function run(action: () => Promise<unknown>) {
    busy = true;
    try {
      await action();
      await load();
    } catch (err) {
      toast.show({ kind: "error", message: errorMessage(err) });
    } finally {
      busy = false;
    }
  }

  // Reload when the client changes (it is read after an await in `run`).
  $effect(() => {
    void clientId;
    untrack(() => run(async () => {}));
  });

  function open(entry?: { contact: Contact; methods: ContactMethod[] }) {
    errors = {};
    const c = entry?.contact;
    editing = {
      id: c?.id ?? null,
      lastName: c?.lastName ?? "",
      firstName: c?.firstName ?? "",
      middleName: c?.middleName ?? "",
      position: c?.position ?? "",
      note: c?.note ?? "",
      isPrimary: c?.isPrimary ?? false,
      methods: entry ? fromMethods(entry.methods) : [],
    };
  }

  const text = (s: string) => (s.trim() === "" ? null : s.trim());

  function validate(d: Draft): Record<string, string> {
    const result: Record<string, string> = {};
    if (!text(d.lastName) && !text(d.firstName)) result.firstName = "validation.required";
    d.methods.forEach((method, i) => {
      const code = methodError(method.kind, method.value);
      if (code) result[`methods.${i}`] = code;
    });
    return result;
  }

  async function save(event: SubmitEvent) {
    event.preventDefault();
    if (!editing) return;
    const draft = editing;
    errors = validate(draft);
    if (Object.keys(errors).length > 0) return;
    const input: ContactInput = {
      lastName: text(draft.lastName),
      firstName: text(draft.firstName),
      middleName: text(draft.middleName),
      position: text(draft.position),
      note: text(draft.note),
      isPrimary: draft.isPrimary,
    };
    busy = true;
    let methodsStep = false;
    try {
      const saved = draft.id
        ? await call(commands.updateContact(draft.id, input))
        : await call(commands.createContact(clientId, input));
      // A retry after a failure in the next step must not create a duplicate.
      draft.id = saved.id;
      methodsStep = true;
      await call(
        commands.replaceContactMethods(
          { type: "contact", id: saved.id },
          toMethodInputs(draft.methods),
        ),
      );
      editing = null;
      await load();
    } catch (err) {
      const index = err instanceof AppError ? err.params.index : undefined;
      if (methodsStep && err instanceof AppError && typeof index === "number") {
        errors = { [`methods.${index}`]: err.code };
      } else {
        toast.show({ kind: "error", message: errorMessage(err) });
      }
    } finally {
      busy = false;
    }
  }

  async function remove(contact: Contact) {
    const ok = await confirm({
      title: m.contact_delete_title({ name: contact.displayName }),
      message: m.contact_delete_message(),
      confirmLabel: m.ui_delete(),
      danger: true,
    });
    if (ok) void run(() => call(commands.deleteContact(contact.id)));
  }

  const error = (name: string) => (errors[name] ? codeMessage(errors[name]) : undefined);
</script>

<div class="flex flex-col gap-4">
  {#if contacts === null}
    <div class="flex justify-center p-6"><Spinner /></div>
  {:else if contacts.length === 0}
    <p class="text-sm text-muted">{m.contacts_empty()}</p>
  {:else}
    <div class="grid gap-3 sm:grid-cols-2">
      {#each contacts as entry (entry.contact.id)}
        {@const c = entry.contact}
        <article class="flex flex-col gap-2 rounded-[var(--radius)] border border-border p-3">
          <div class="flex items-start gap-2">
            <div class="min-w-0 flex-1">
              <h3 class="truncate text-sm font-semibold text-fg">{c.displayName}</h3>
              {#if c.position}<p class="text-xs text-muted">{c.position}</p>{/if}
            </div>
            {#if c.isPrimary}
              <span class="rounded-full bg-surface px-2 py-0.5 text-xs font-medium text-accent">
                {m.contact_primary()}
              </span>
            {/if}
          </div>
          {#if entry.methods.length > 0}
            <dl class="grid grid-cols-[auto_1fr] gap-x-3 gap-y-1 text-sm">
              {#each entry.methods as method (method.id)}
                <dt class="text-muted">{method.label ?? methodLabel(method.kind)}</dt>
                <dd class="break-all text-fg">{method.value}</dd>
              {/each}
            </dl>
          {/if}
          {#if c.note}<p class="text-sm whitespace-pre-wrap text-muted">{c.note}</p>{/if}
          <div class="mt-auto flex gap-1">
            {#if !c.isPrimary}
              <Button
                variant="ghost"
                size="sm"
                aria-label={m.contact_make_primary()}
                disabled={busy}
                onclick={() => run(() => call(commands.setPrimaryContact(c.id)))}
              >
                <Star class="size-4" aria-hidden="true" />
              </Button>
            {/if}
            <Button variant="ghost" size="sm" aria-label={m.ui_edit()} onclick={() => open(entry)}>
              <Pencil class="size-4" aria-hidden="true" />
            </Button>
            <Button variant="ghost" size="sm" aria-label={m.ui_delete()} onclick={() => remove(c)}>
              <Trash2 class="size-4" aria-hidden="true" />
            </Button>
          </div>
        </article>
      {/each}
    </div>
  {/if}
  <div>
    <Button onclick={() => open()}>
      <Plus class="size-4" aria-hidden="true" />
      {m.contacts_add()}
    </Button>
  </div>
</div>

{#if editing}
  <Dialog
    open
    title={editing.id ? m.contact_edit() : m.contact_new()}
    onclose={() => (editing = null)}
  >
    <form id={formId} class="flex w-[40rem] max-w-full flex-col gap-4" novalidate onsubmit={save}>
      <div class="grid gap-4 sm:grid-cols-3">
        <TextField label={m.client_field_last_name()} bind:value={editing.lastName} />
        <TextField
          label={m.client_field_first_name()}
          bind:value={editing.firstName}
          error={error("firstName")}
        />
        <TextField label={m.client_field_middle_name()} bind:value={editing.middleName} />
      </div>
      <TextField label={m.contact_field_position()} bind:value={editing.position} />
      <Checkbox label={m.contact_field_primary()} bind:checked={editing.isPrimary} />
      <ContactMethodsEditor
        bind:methods={editing.methods}
        errors={methodErrors}
        onremove={() => (errors = {})}
      />
      <TextArea label={m.contact_field_note()} rows={3} bind:value={editing.note} />
    </form>
    {#snippet footer()}
      <Button onclick={() => (editing = null)}>{m.ui_cancel()}</Button>
      <Button type="submit" form={formId} variant="primary" loading={busy}>{m.ui_save()}</Button>
    {/snippet}
  </Dialog>
{/if}
