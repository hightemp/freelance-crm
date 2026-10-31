<script lang="ts">
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { i18n } from "$lib/i18n/locale.svelte";
  import { renderMarkdown } from "./render";

  let {
    source,
    breaks = true,
  }: {
    source: string;
    /** Single line breaks in the source break lines (notes), else wrap. */
    breaks?: boolean;
  } = $props();

  // Re-render on locale change: task marks carry translated labels.
  let html = $derived.by(() => {
    void i18n.locale;
    return renderMarkdown(source, { breaks });
  });

  /** Links never navigate the app window; web and mail links open outside. */
  function onclick(event: MouseEvent) {
    const link = (event.target as Element).closest("a");
    if (!link) return;
    event.preventDefault();
    const href = link.getAttribute("href") ?? "";
    if (/^(https?:|mailto:)/i.test(href)) void openUrl(href);
  }
</script>

<!-- Clicks are delegated from the links inside, which are keyboard-operable. -->
<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="markdown" {onclick}>
  <!-- Sanitized by renderMarkdown (DOMPurify). -->
  <!-- eslint-disable-next-line svelte/no-at-html-tags -->
  {@html html}
</div>
