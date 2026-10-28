<script lang="ts">
  import { m } from "$lib/paraglide/messages";
  import Button from "./Button.svelte";

  let {
    label,
    onPick,
    onDrop,
  }: {
    label: string;
    /** Opens the OS file dialog (Tauri), replacing input[type=file]. */
    onPick: () => void;
    onDrop: (files: File[]) => void;
  } = $props();

  let dragging = $state(false);
  let depth = 0; // dragenter/leave fire for children too

  function hasFiles(event: DragEvent) {
    return [...(event.dataTransfer?.types ?? [])].includes("Files");
  }
</script>

<div
  role="presentation"
  data-testid="drop-zone"
  data-dragging={dragging}
  ondragenter={(e) => {
    if (!hasFiles(e)) return;
    e.preventDefault();
    depth++;
    dragging = true;
  }}
  ondragover={(e) => hasFiles(e) && e.preventDefault()}
  ondragleave={() => {
    depth = Math.max(0, depth - 1);
    if (depth === 0) dragging = false;
  }}
  ondrop={(e) => {
    e.preventDefault();
    depth = 0;
    dragging = false;
    onDrop([...(e.dataTransfer?.files ?? [])]);
  }}
  class="flex flex-col items-center gap-2 rounded-[var(--radius)] border border-dashed border-border
    p-6 text-sm text-muted data-[dragging=true]:border-accent data-[dragging=true]:bg-surface"
>
  <Button onclick={onPick}>{label}</Button>
  <span>{m.ui_drop_files()}</span>
</div>
