<script lang="ts">
  import CalendarPage from "$lib/features/calendar/CalendarPage.svelte";
  import { m } from "$lib/paraglide/messages";
</script>

<div class="flex flex-col gap-6 p-6">
  <h1 class="text-xl font-semibold text-fg">{m.nav_calendar()}</h1>
  <CalendarPage />
</div>
