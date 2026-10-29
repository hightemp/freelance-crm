<script lang="ts">
  import type { EditScope } from "$lib/api/bindings";
  import { m } from "$lib/paraglide/messages";
  import Button from "$lib/ui/Button.svelte";
  import Dialog from "$lib/ui/Dialog.svelte";
  import RadioGroup from "$lib/ui/RadioGroup.svelte";

  let {
    title,
    onchoose,
    oncancel,
  }: { title: string; onchoose: (scope: EditScope) => void; oncancel: () => void } = $props();

  let scope = $state<EditScope>("this");
  const options = [
    { value: "this", label: m.scope_this() },
    { value: "following", label: m.scope_following() },
    { value: "all", label: m.scope_all() },
  ];
</script>

<Dialog open {title} onclose={oncancel}>
  <RadioGroup
    label={m.scope_label()}
    {options}
    bind:value={() => scope, (v) => (scope = v as EditScope)}
  />
  {#snippet footer()}
    <Button onclick={oncancel}>{m.ui_cancel()}</Button>
    <Button variant="primary" onclick={() => onchoose(scope)}>{m.ui_ok()}</Button>
  {/snippet}
</Dialog>
