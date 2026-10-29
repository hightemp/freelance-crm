<script lang="ts">
  import ChevronLeft from "@lucide/svelte/icons/chevron-left";
  import ChevronRight from "@lucide/svelte/icons/chevron-right";
  import type { Attachment } from "$lib/api/bindings";
  import { m } from "$lib/paraglide/messages";
  import Button from "$lib/ui/Button.svelte";
  import Dialog from "$lib/ui/Dialog.svelte";
  import { previewUrl } from "./preview";

  let {
    images,
    currentId,
    onnavigate,
    onclose,
  }: {
    images: Attachment[];
    currentId: string;
    onnavigate: (id: string) => void;
    onclose: () => void;
  } = $props();

  let index = $derived(images.findIndex((f) => f.id === currentId));
  let current = $derived(images[index]);

  function step(delta: number) {
    onnavigate(images[(index + delta + images.length) % images.length].id);
  }

  function onkeydown(event: KeyboardEvent) {
    if (event.key === "ArrowRight") step(1);
    else if (event.key === "ArrowLeft") step(-1);
    else return;
    event.preventDefault();
  }
</script>

<svelte:window {onkeydown} />

{#if current}
  <Dialog open title={current.fileName} {onclose}>
    <div class="flex w-[min(90vw,64rem)] flex-col items-center gap-3">
      <img
        src={previewUrl("full", current.id)}
        alt={current.fileName}
        class="max-h-[75vh] max-w-full object-contain"
      />
      {#if images.length > 1}
        <div class="flex items-center gap-2">
          <Button variant="ghost" aria-label={m.files_previous()} onclick={() => step(-1)}>
            <ChevronLeft class="size-5" aria-hidden="true" />
          </Button>
          <span class="text-sm text-muted tabular-nums">{index + 1} / {images.length}</span>
          <Button variant="ghost" aria-label={m.files_next()} onclick={() => step(1)}>
            <ChevronRight class="size-5" aria-hidden="true" />
          </Button>
        </div>
      {/if}
    </div>
  </Dialog>
{/if}
