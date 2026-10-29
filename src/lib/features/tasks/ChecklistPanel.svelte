<script lang="ts">
  import { untrack } from "svelte";
  import ArrowUp from "@lucide/svelte/icons/arrow-up";
  import Trash2 from "@lucide/svelte/icons/trash-2";
  import { commands, type ChecklistItem } from "$lib/api/bindings";
  import { call } from "$lib/api/client";
  import { errorMessage } from "$lib/i18n/errors";
  import { m } from "$lib/paraglide/messages";
  import Button from "$lib/ui/Button.svelte";
  import Checkbox from "$lib/ui/Checkbox.svelte";
  import Progress from "$lib/ui/Progress.svelte";
  import Spinner from "$lib/ui/Spinner.svelte";
  import TextField from "$lib/ui/TextField.svelte";
  import { toast } from "$lib/ui/toast.svelte";

  let {
    taskId,
    onchange,
  }: {
    taskId: string;
    /** Called after any change, e.g. to refresh the task's progress. */
    onchange?: () => void;
  } = $props();

  let items = $state<ChecklistItem[] | null>(null);
  let draft = $state("");
  let busy = $state(false);

  let done = $derived(items?.filter((i) => i.done).length ?? 0);
  // Same rounding as crm_core: whole percent, rounded down.
  let percent = $derived(items?.length ? Math.floor((done * 100) / items.length) : null);

  async function run(action: () => Promise<unknown>, changed = true) {
    busy = true;
    try {
      await action();
      items = await call(commands.listChecklist(taskId));
      if (changed) onchange?.();
    } catch (err) {
      toast.show({ kind: "error", message: errorMessage(err) });
    } finally {
      busy = false;
    }
  }

  $effect(() => {
    void taskId;
    untrack(() => run(async () => {}, false));
  });

  function add(event: SubmitEvent) {
    event.preventDefault();
    const text = draft.trim();
    if (!text) return;
    void run(async () => {
      await call(commands.addChecklistItem(taskId, text));
      draft = "";
    });
  }

  function moveUp(index: number) {
    if (!items || index === 0) return;
    const ids = items.map((i) => i.id);
    [ids[index - 1], ids[index]] = [ids[index], ids[index - 1]];
    void run(() => call(commands.reorderChecklist(taskId, ids)));
  }
</script>

<section class="flex flex-col gap-3" aria-labelledby="checklist-{taskId}">
  <h2 id="checklist-{taskId}" class="text-base font-semibold text-fg">{m.checklist()}</h2>
  {#if items === null}
    <div class="flex justify-center p-4"><Spinner /></div>
  {:else}
    {#if percent !== null}
      <div class="flex items-center gap-3">
        <div class="flex-1"><Progress label={m.checklist()} value={percent} /></div>
        <span class="text-xs text-muted tabular-nums">
          {m.checklist_progress({ percent })}
        </span>
      </div>
    {/if}
    {#if items.length === 0}
      <p class="text-sm text-muted">{m.checklist_empty()}</p>
    {:else}
      <ul class="flex flex-col gap-1">
        {#each items as item, i (item.id)}
          <li class="flex items-center gap-2">
            <div class="min-w-0 flex-1 {item.done ? 'text-muted line-through' : ''}">
              <Checkbox
                label={item.text}
                bind:checked={
                  () => item.done,
                  (value) =>
                    run(() => call(commands.updateChecklistItem(item.id, item.text, value)))
                }
              />
            </div>
            <Button
              variant="ghost"
              size="sm"
              aria-label={m.field_move_up()}
              disabled={busy || i === 0}
              onclick={() => moveUp(i)}
            >
              <ArrowUp class="size-4" aria-hidden="true" />
            </Button>
            <Button
              variant="ghost"
              size="sm"
              aria-label={m.checklist_delete({ text: item.text })}
              disabled={busy}
              onclick={() => run(() => call(commands.deleteChecklistItem(item.id)))}
            >
              <Trash2 class="size-4" aria-hidden="true" />
            </Button>
          </li>
        {/each}
      </ul>
    {/if}
    <form class="flex items-end gap-2" onsubmit={add}>
      <div class="flex-1"><TextField label={m.checklist_new()} bind:value={draft} /></div>
      <!-- Not disabled when empty: a disabled default button blocks Enter
           (implicit submission); empty input is ignored by `add`. -->
      <Button type="submit" disabled={busy}>{m.checklist_add()}</Button>
    </form>
  {/if}
</section>
