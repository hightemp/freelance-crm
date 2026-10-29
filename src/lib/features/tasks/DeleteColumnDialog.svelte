<script lang="ts">
  import type { TaskStatus_Serialize as TaskStatus } from "$lib/api/bindings";
  import { m } from "$lib/paraglide/messages";
  import Button from "$lib/ui/Button.svelte";
  import Dialog from "$lib/ui/Dialog.svelte";
  import Select from "$lib/ui/Select.svelte";
  import { statusName } from "./statuses";

  let {
    column,
    taskCount,
    others,
    busy = false,
    ondelete,
    oncancel,
  }: {
    column: TaskStatus;
    taskCount: number;
    /** Columns the tasks can be moved to. */
    others: TaskStatus[];
    busy?: boolean;
    ondelete: (moveTo: string | null) => void;
    oncancel: () => void;
  } = $props();

  let target = $state<string | null>(null);
  let options = $derived(others.map((c) => ({ value: c.id, label: statusName(c) })));
</script>

<Dialog
  open
  role="alertdialog"
  title={m.column_delete_title({ name: statusName(column) })}
  onclose={oncancel}
>
  <div class="w-80 max-w-full">
    {#if taskCount > 0}
      <Select
        label={m.column_delete_move({ count: taskCount })}
        {options}
        required
        bind:value={target}
      />
    {:else}
      <p class="text-sm text-fg">{m.column_delete_empty()}</p>
    {/if}
  </div>
  {#snippet footer()}
    <Button data-autofocus onclick={oncancel}>{m.ui_cancel()}</Button>
    <Button
      variant="danger"
      loading={busy}
      disabled={taskCount > 0 && !target}
      onclick={() => ondelete(taskCount > 0 ? target : null)}
    >
      {m.ui_delete()}
    </Button>
  {/snippet}
</Dialog>
