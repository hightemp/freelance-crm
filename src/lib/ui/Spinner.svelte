<script lang="ts">
  import { m } from "$lib/paraglide/messages";

  let { size = 16 }: { size?: number } = $props();
</script>

<span role="status" aria-label={m.ui_loading()} class="inline-flex">
  <svg
    class="animate-spin"
    width={size}
    height={size}
    viewBox="0 0 24 24"
    fill="none"
    aria-hidden="true"
  >
    <circle cx="12" cy="12" r="10" stroke="currentColor" stroke-opacity="0.25" stroke-width="3" />
    <path d="M22 12a10 10 0 0 0-10-10" stroke="currentColor" stroke-width="3" />
  </svg>
</span>
