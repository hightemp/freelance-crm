<script lang="ts">
  import Printer from "@lucide/svelte/icons/printer";
  import Save from "@lucide/svelte/icons/save";
  import { save } from "@tauri-apps/plugin-dialog";
  import { commands } from "$lib/api/bindings";
  import { call } from "$lib/api/client";
  import { errorMessage } from "$lib/i18n/errors";
  import { m } from "$lib/paraglide/messages";
  import Button from "$lib/ui/Button.svelte";
  import TextField from "$lib/ui/TextField.svelte";
  import { toast } from "$lib/ui/toast.svelte";

  let {
    recoveryKey,
    oncomplete,
    pickGroups = randomGroups,
  }: {
    recoveryKey: string;
    oncomplete: () => void;
    /** Zero-based groups to ask for (random by default). */
    pickGroups?: (count: number) => [number, number];
  } = $props();

  function randomGroups(count: number): [number, number] {
    const first = Math.floor(Math.random() * count);
    const second = (first + 1 + Math.floor(Math.random() * (count - 1))) % count;
    return first < second ? [first, second] : [second, first];
  }

  let groups = $derived(recoveryKey.split("-"));
  let step = $state<"show" | "confirm">("show");
  let asked = $state<[number, number]>([0, 1]);
  let answers = $state(["", ""]);
  let wrong = $state(false);

  async function saveToFile() {
    try {
      const path = await save({
        defaultPath: "freelance-crm-recovery-key.txt",
        filters: [{ name: "Text", extensions: ["txt"] }],
      });
      if (!path) return;
      await call(commands.saveRecoveryKeyFile(path, recoveryKey));
      toast.show({ message: m.recovery_saved_toast() });
    } catch (err) {
      toast.show({ kind: "error", message: errorMessage(err) });
    }
  }

  function startConfirm() {
    asked = pickGroups(groups.length);
    answers = ["", ""];
    wrong = false;
    step = "confirm";
  }

  function confirmGroups(event: SubmitEvent) {
    event.preventDefault();
    const ok = asked.every(
      (group, i) => answers[i].trim().toUpperCase() === groups[group].toUpperCase(),
    );
    wrong = !ok;
    if (ok) oncomplete();
  }
</script>

{#if step === "show"}
  <section class="flex max-w-xl flex-col gap-4" aria-labelledby="recovery-title">
    <h2 id="recovery-title" class="text-base font-semibold text-fg">{m.recovery_title()}</h2>
    <p class="text-sm text-fg">{m.recovery_hint()}</p>
    <div
      role="group"
      aria-label={m.recovery_key_label()}
      class="print-area grid grid-cols-7 gap-2 rounded-[var(--radius)] border border-border bg-surface p-4 font-mono text-base tracking-wider text-fg"
    >
      <!-- A trailing space keeps groups apart when copied or read aloud. -->
      {#each groups as group, i (i)}
        <span>{i < groups.length - 1 ? `${group} ` : group}</span>
      {/each}
    </div>
    <div class="flex flex-wrap gap-2">
      <Button onclick={saveToFile}>
        <Save class="size-4" aria-hidden="true" />
        {m.recovery_save_file()}
      </Button>
      <Button onclick={() => window.print()}>
        <Printer class="size-4" aria-hidden="true" />
        {m.recovery_print()}
      </Button>
      <Button variant="primary" onclick={startConfirm}>{m.recovery_continue()}</Button>
    </div>
  </section>
{:else}
  <form class="flex max-w-md flex-col gap-4" novalidate onsubmit={confirmGroups}>
    <h2 class="text-base font-semibold text-fg">{m.recovery_confirm_title()}</h2>
    <p class="text-sm text-muted">
      {m.recovery_confirm_hint({ first: asked[0] + 1, second: asked[1] + 1 })}
    </p>
    <div class="grid grid-cols-2 gap-4">
      {#each asked as group, i (group)}
        <TextField
          label={m.recovery_group({ number: group + 1 })}
          autocomplete="off"
          spellcheck="false"
          bind:value={answers[i]}
        />
      {/each}
    </div>
    {#if wrong}
      <p class="text-sm text-danger" role="alert">{m.recovery_confirm_wrong()}</p>
    {/if}
    <div class="flex gap-2">
      <Button type="submit" variant="primary">{m.recovery_confirm()}</Button>
      <Button onclick={() => (step = "show")}>{m.recovery_back()}</Button>
    </div>
  </form>
{/if}
