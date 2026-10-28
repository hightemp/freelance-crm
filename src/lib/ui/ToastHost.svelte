<script lang="ts">
  import { m } from "$lib/paraglide/messages";
  import { toast, visibleToasts } from "./toast.svelte";

  let toasts = $derived(visibleToasts());
</script>

<section
  aria-label={m.ui_notifications()}
  class="pointer-events-none fixed right-4 bottom-4 z-[60] flex w-80 flex-col gap-2"
>
  {#each toasts as t (t.id)}
    <div
      role={t.kind === "error" ? "alert" : "status"}
      class="pointer-events-auto flex items-start gap-2 rounded-[var(--radius)] border bg-bg p-3
        text-sm text-fg shadow-lg {t.kind === 'error' ? 'border-danger' : 'border-border'}"
    >
      <p class="flex-1">{t.message}</p>
      <button
        type="button"
        aria-label={m.ui_dismiss()}
        onclick={() => toast.dismiss(t.id)}
        class="grid size-5 place-items-center rounded text-muted hover:text-fg"
      >
        <svg width="10" height="10" viewBox="0 0 12 12" aria-hidden="true">
          <path d="M2 2l8 8M10 2l-8 8" stroke="currentColor" stroke-width="1.5" />
        </svg>
      </button>
    </div>
  {/each}
</section>
