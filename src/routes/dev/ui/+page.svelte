<script lang="ts">
  // `import.meta.env.DEV` is replaced at build time, so production builds
  // drop this dynamic import and its chunk entirely (`pnpm check:build`).
  const showcase = import.meta.env.DEV ? import("$lib/dev/Showcase.svelte") : null;
</script>

{#if showcase}
  {#await showcase then { default: Showcase }}
    <Showcase />
  {/await}
{/if}
