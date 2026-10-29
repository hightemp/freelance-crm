<script lang="ts">
  import type { DeleteImpact } from "$lib/api/bindings";
  import { m } from "$lib/paraglide/messages";
  import Button from "$lib/ui/Button.svelte";
  import Dialog from "$lib/ui/Dialog.svelte";

  let {
    name,
    impact,
    archived,
    busy = false,
    ondelete,
    onarchive,
    oncancel,
  }: {
    name: string;
    impact: DeleteImpact;
    /** Already archived clients are not offered archiving. */
    archived: boolean;
    busy?: boolean;
    ondelete: () => void;
    onarchive: () => void;
    oncancel: () => void;
  } = $props();

  let rows = $derived(
    [
      { label: m.client_tab_contacts(), count: impact.contacts },
      { label: m.client_tab_projects(), count: impact.projects },
      { label: m.contact_methods(), count: impact.contactMethods },
      { label: m.client_tab_fields(), count: impact.customFields },
      { label: m.client_tab_notes(), count: impact.notes },
    ].filter((row) => row.count > 0),
  );
</script>

<Dialog open role="alertdialog" title={m.client_delete_title({ name })} onclose={oncancel}>
  <div class="flex max-w-md flex-col gap-3 text-sm text-fg">
    {#if rows.length > 0}
      <p>{m.client_delete_impact()}</p>
      <ul class="flex flex-col gap-1">
        {#each rows as row (row.label)}
          <li class="flex justify-between gap-6">
            <span>{row.label}</span>
            <span class="tabular-nums">{row.count}</span>
          </li>
        {/each}
      </ul>
    {:else}
      <p>{m.client_delete_nothing()}</p>
    {/if}
    <p class="text-muted">{m.client_delete_warning()}</p>
  </div>
  {#snippet footer()}
    <Button data-autofocus onclick={oncancel}>{m.ui_cancel()}</Button>
    {#if !archived}
      <Button disabled={busy} onclick={onarchive}>{m.client_archive_instead()}</Button>
    {/if}
    <Button variant="danger" loading={busy} onclick={ondelete}>{m.ui_delete()}</Button>
  {/snippet}
</Dialog>
