<script lang="ts">
  import Monitor from "@lucide/svelte/icons/monitor";
  import Moon from "@lucide/svelte/icons/moon";
  import Sun from "@lucide/svelte/icons/sun";
  import { resolve } from "$app/paths";
  import { m } from "$lib/paraglide/messages";
  import DropdownMenu from "$lib/ui/DropdownMenu.svelte";
  import { NAV_ITEMS, isActive } from "./nav";
  import { setTheme, theme, type ThemeMode } from "./theme.svelte";

  let { currentPath }: { currentPath: string } = $props();

  const themeIcons = { light: Sun, dark: Moon, system: Monitor };
  let ThemeIcon = $derived(themeIcons[theme.mode]);
</script>

<aside class="flex h-full w-52 flex-col border-r border-border bg-surface">
  <div class="px-4 py-3 text-sm font-semibold text-fg">{m.app_title()}</div>
  <nav aria-label={m.nav_main()} class="flex-1 overflow-y-auto px-2">
    <ul class="flex flex-col gap-0.5">
      {#each NAV_ITEMS as item (item.href)}
        {@const active = isActive(item.href, currentPath)}
        <li>
          <a
            href={resolve(item.href)}
            aria-current={active ? "page" : undefined}
            class="flex items-center gap-2.5 rounded-[var(--radius)] px-2.5 py-1.5 text-sm
              text-muted hover:bg-surface-hover hover:text-fg
              aria-[current=page]:bg-surface-hover aria-[current=page]:font-medium
              aria-[current=page]:text-fg"
          >
            <item.icon size={16} aria-hidden="true" />
            {item.label()}
          </a>
        </li>
      {/each}
    </ul>
  </nav>
  <div class="border-t border-border p-2">
    <DropdownMenu
      placement="top-start"
      items={[
        { id: "light", label: m.theme_light() },
        { id: "dark", label: m.theme_dark() },
        { id: "system", label: m.theme_system() },
      ]}
      onSelect={(id) => setTheme(id as ThemeMode)}
    >
      {#snippet trigger(props)}
        <button
          {...props}
          type="button"
          aria-label={m.ui_theme()}
          class="grid size-8 place-items-center rounded-[var(--radius)] text-muted
            hover:bg-surface-hover hover:text-fg"
        >
          <ThemeIcon size={16} aria-hidden="true" />
        </button>
      {/snippet}
    </DropdownMenu>
  </div>
</aside>
