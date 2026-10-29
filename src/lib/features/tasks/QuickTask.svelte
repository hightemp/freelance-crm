<script lang="ts">
  import { commands } from "$lib/api/bindings";
  import { call } from "$lib/api/client";
  import { codeMessage, errorMessage } from "$lib/i18n/errors";
  import { m } from "$lib/paraglide/messages";
  import Button from "$lib/ui/Button.svelte";
  import Dialog from "$lib/ui/Dialog.svelte";
  import TextField from "$lib/ui/TextField.svelte";
  import { toast } from "$lib/ui/toast.svelte";

  // Global quick capture (FR-TS-7): Ctrl+N / Cmd+N from any screen opens a
  // one-field dialog; Enter files the task into the inbox.

  const formId = $props.id();
  let open = $state(false);
  let title = $state("");
  let error = $state<string | null>(null);
  let saving = $state(false);

  function onkeydown(event: KeyboardEvent) {
    const mod = event.ctrlKey || event.metaKey;
    if (!mod || event.altKey || event.shiftKey || event.key.toLowerCase() !== "n") return;
    event.preventDefault();
    title = "";
    error = null;
    open = true;
  }

  async function create(event: SubmitEvent) {
    event.preventDefault();
    const text = title.trim();
    if (!text) {
      error = "validation.required";
      return;
    }
    saving = true;
    try {
      const task = await call(
        commands.createTask({
          title: text,
          descriptionMd: "",
          statusId: null,
          priority: "normal",
          projectId: null,
          clientId: null,
          dueDate: null,
          dueTime: null,
          estimateMinutes: null,
        }),
      );
      open = false;
      toast.show({ message: m.task_created_toast({ title: task.title }) });
    } catch (err) {
      toast.show({ kind: "error", message: errorMessage(err) });
    } finally {
      saving = false;
    }
  }
</script>

<svelte:window {onkeydown} />

{#if open}
  <Dialog
    open
    title={m.quick_task_title()}
    description={m.quick_task_hint()}
    onclose={() => (open = false)}
  >
    <form id={formId} class="w-96 max-w-full" novalidate onsubmit={create}>
      <TextField
        label={m.task_field_title()}
        bind:value={title}
        error={error ? codeMessage(error) : undefined}
        oninput={() => (error = null)}
      />
    </form>
    {#snippet footer()}
      <Button onclick={() => (open = false)}>{m.ui_cancel()}</Button>
      <Button type="submit" form={formId} variant="primary" loading={saving}>
        {m.ui_save()}
      </Button>
    {/snippet}
  </Dialog>
{/if}
