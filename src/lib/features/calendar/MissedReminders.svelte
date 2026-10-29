<script lang="ts">
  import { listen } from "@tauri-apps/api/event";
  import { commands, type Candidate } from "$lib/api/bindings";
  import { call } from "$lib/api/client";
  import { errorMessage } from "$lib/i18n/errors";
  import { i18n } from "$lib/i18n/locale.svelte";
  import { m } from "$lib/paraglide/messages";
  import Button from "$lib/ui/Button.svelte";
  import Dialog from "$lib/ui/Dialog.svelte";
  import { toast } from "$lib/ui/toast.svelte";

  /** Emitted by the backend when it finds reminders missed while closed. */
  const MISSED_EVENT = "reminders://missed";

  let missed = $state<Candidate[]>([]);
  /** Closed for now; shown again when more are missed. */
  let dismissed = $state(false);

  const showError = (err: unknown) => toast.show({ kind: "error", message: errorMessage(err) });

  async function load() {
    missed = await call(commands.missedReminders());
    dismissed = false;
  }

  $effect(() => {
    load().catch(showError);
    let unlisten: (() => void) | undefined;
    let stopped = false;
    // Unlistening may fail without an event system (tests): never break.
    const safely = (fn: () => void) =>
      void Promise.resolve()
        .then(fn)
        .catch(() => {});
    listen(MISSED_EVENT, () => load().catch(showError)).then(
      (fn) => (stopped ? safely(fn) : (unlisten = fn)),
      () => {},
    );
    return () => {
      stopped = true;
      if (unlisten) safely(unlisten);
    };
  });

  function when(c: Candidate): string {
    // All-day items are floating dates.
    return new Intl.DateTimeFormat(
      i18n.locale,
      c.allDay
        ? { dateStyle: "medium", timeZone: "UTC" }
        : { dateStyle: "medium", timeStyle: "short" },
    ).format(c.startAt);
  }

  async function markSeen() {
    try {
      await call(commands.markMissedRemindersSeen());
      missed = [];
    } catch (err) {
      showError(err);
    }
  }
</script>

{#if missed.length > 0 && !dismissed}
  <Dialog open title={m.reminders_missed_title()} onclose={() => (dismissed = true)}>
    <ul class="flex w-[28rem] max-w-full flex-col gap-2">
      {#each missed as c (c.reminderId + c.occurrence + c.fireAt)}
        <li class="flex justify-between gap-4 text-sm">
          <span class="min-w-0 truncate text-fg">{c.title}</span>
          <span class="shrink-0 text-muted">{when(c)}</span>
        </li>
      {/each}
    </ul>
    {#snippet footer()}
      <Button variant="primary" onclick={markSeen}>{m.reminders_mark_seen()}</Button>
    {/snippet}
  </Dialog>
{/if}
