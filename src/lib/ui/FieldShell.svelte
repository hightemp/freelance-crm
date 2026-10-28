<script lang="ts">
  import type { Snippet } from "svelte";

  // Label, hint and error layout shared by text-like fields.
  let {
    id,
    label,
    hint,
    error,
    required = false,
    children,
  }: {
    id: string;
    label: string;
    hint?: string;
    error?: string;
    required?: boolean;
    children: Snippet;
  } = $props();
</script>

<div class="flex flex-col gap-1">
  <label for={id} class="text-sm font-medium text-fg">
    {label}{#if required}<span class="text-danger" aria-hidden="true"> *</span>{/if}
  </label>
  {@render children()}
  {#if hint}<p id="{id}-hint" class="text-xs text-muted">{hint}</p>{/if}
  {#if error}<p id="{id}-error" class="text-xs text-danger">{error}</p>{/if}
</div>
