<script lang="ts">
  import "../app.css";
  import { onMount } from "svelte";
  import EditMenuHost from "$lib/app/EditMenuHost.svelte";
  import { openEditMenu } from "$lib/app/edit-menu.svelte";
  import { installBrowserGuards } from "$lib/app/guards";
  import { stepZoom } from "$lib/app/zoom.svelte";
  import DialogHost from "$lib/ui/DialogHost.svelte";
  import ToastHost from "$lib/ui/ToastHost.svelte";

  let { children } = $props();

  onMount(() =>
    installBrowserGuards({
      dev: import.meta.env.DEV,
      onFind: () => {}, // global search arrives in M10 (FR-SR-1)
      onZoom: stepZoom,
      onEditableContextMenu: openEditMenu,
    }),
  );
</script>

{@render children()}

<DialogHost />
<ToastHost />
<EditMenuHost />
