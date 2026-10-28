<script lang="ts">
  import DatePicker from "./DatePicker.svelte";

  let {
    startLabel,
    endLabel,
    start = $bindable(null),
    end = $bindable(null),
    min,
    max,
  }: {
    startLabel: string;
    endLabel: string;
    start?: string | null;
    end?: string | null;
    min?: string;
    max?: string;
  } = $props();
</script>

<div class="flex items-start gap-2">
  <div class="flex-1">
    <DatePicker label={startLabel} bind:value={start} {min} max={end ?? max} />
  </div>
  <div class="flex-1">
    <DatePicker label={endLabel} bind:value={end} min={start ?? min} {max} />
  </div>
</div>
