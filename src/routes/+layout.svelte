<script lang="ts">
  import "../app.css";
  import { onMount } from "svelte";
  import { page } from "$app/state";
  import AppSidebar from "$lib/app/AppSidebar.svelte";
  import EditMenuHost from "$lib/app/EditMenuHost.svelte";
  import QuickTask from "$lib/features/tasks/QuickTask.svelte";
  import { openEditMenu } from "$lib/app/edit-menu.svelte";
  import { commands } from "$lib/api/bindings";
  import { installActivityReporter } from "$lib/app/activity";
  import { installBrowserGuards } from "$lib/app/guards";
  import { loadSettings, updateSettings } from "$lib/app/settings.svelte";
  import { initTheme } from "$lib/app/theme.svelte";
  import { stepZoom, zoom } from "$lib/app/zoom.svelte";
  import MissedReminders from "$lib/features/calendar/MissedReminders.svelte";
  import TimerWidget from "$lib/features/time/TimerWidget.svelte";
  import DialogHost from "$lib/ui/DialogHost.svelte";
  import ToastHost from "$lib/ui/ToastHost.svelte";

  let { children } = $props();

  onMount(() => {
    initTheme();
    void loadSettings();
    const removeGuards = installBrowserGuards({
      dev: import.meta.env.DEV,
      onFind: () => {}, // global search arrives in M10 (FR-SR-1)
      onZoom: (step) => {
        stepZoom(step);
        void updateSettings({ zoomPercent: Math.round(zoom.level * 100) });
      },
      onEditableContextMenu: openEditMenu,
    });
    // Keeps the vault from auto-locking while the user works.
    const removeActivity = installActivityReporter(() => void commands.vaultTouch(), 30_000);
    return () => {
      removeGuards();
      removeActivity();
    };
  });
</script>

<div class="flex h-screen overflow-hidden">
  <AppSidebar currentPath={page.url.pathname} />
  <div class="flex min-w-0 flex-1 flex-col">
    <header class="flex h-12 shrink-0 items-center justify-end gap-2 border-b border-border px-4">
      <TimerWidget />
    </header>
    <main class="flex-1 overflow-y-auto">
      {@render children()}
    </main>
  </div>
</div>

<DialogHost />
<MissedReminders />
<ToastHost />
<EditMenuHost />
<QuickTask />
