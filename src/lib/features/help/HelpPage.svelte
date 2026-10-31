<script lang="ts">
  import { i18n } from "$lib/i18n/locale.svelte";
  import Markdown from "$lib/markdown/Markdown.svelte";
  import { m } from "$lib/paraglide/messages";
  import TextField from "$lib/ui/TextField.svelte";
  import { GUIDES, matches } from "./guide";

  let query = $state("");
  let guide = $derived(GUIDES[i18n.locale === "ru" ? "ru" : "en"]);
  let shown = $derived(guide.sections.filter((s) => matches(s, query)));

  function show(id: string) {
    document.getElementById(id)?.scrollIntoView({ block: "start" });
  }
</script>

<div class="flex flex-col gap-6 p-6">
  <h1 class="text-xl font-semibold text-fg">{guide.title}</h1>

  <div class="grid items-start gap-8 md:grid-cols-[14rem_minmax(0,1fr)]">
    <!-- Stays in view while the guide scrolls. -->
    <div class="flex flex-col gap-4 md:sticky md:top-0">
      <TextField label={m.help_search()} type="search" bind:value={query} />
      <nav aria-label={m.help_contents()}>
        <ul class="flex flex-col gap-0.5">
          {#each shown as section (section.id)}
            <li>
              <button
                type="button"
                class="w-full rounded px-2 py-1 text-left text-sm text-muted hover:bg-surface-hover hover:text-fg"
                onclick={() => show(section.id)}
              >
                {section.title}
              </button>
            </li>
          {/each}
        </ul>
      </nav>
    </div>

    <div class="flex max-w-3xl flex-col gap-8">
      {#if query.trim() === ""}
        <Markdown source={guide.intro} breaks={false} />
      {/if}
      {#each shown as section (section.id)}
        <section id={section.id} aria-labelledby="{section.id}-title" class="scroll-mt-4">
          <h2 id="{section.id}-title" class="mb-3 text-base font-semibold text-fg">
            {section.title}
          </h2>
          <Markdown source={section.body} breaks={false} />
        </section>
      {:else}
        <p class="text-sm text-muted">{m.help_nothing_found()}</p>
      {/each}
    </div>
  </div>
</div>
