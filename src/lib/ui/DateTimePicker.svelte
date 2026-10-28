<script lang="ts">
  import DatePicker from "./DatePicker.svelte";
  import TimePicker from "./TimePicker.svelte";

  let {
    dateLabel,
    timeLabel,
    value = $bindable(null),
    min,
    max,
  }: {
    dateLabel: string;
    timeLabel: string;
    /** Local date-time `YYYY-MM-DDTHH:MM`; null until both parts are set. */
    value?: string | null;
    min?: string;
    max?: string;
  } = $props();

  let date = $state<string | null>(null);
  let time = $state<string | null>(null);

  // Pick up external changes of `value`.
  $effect.pre(() => {
    if (value) [date, time] = value.split("T");
  });

  function update(nextDate: string | null, nextTime: string | null) {
    date = nextDate;
    time = nextTime;
    value = date && time ? `${date}T${time}` : null;
  }
</script>

<div class="flex items-start gap-2">
  <div class="flex-1">
    <DatePicker label={dateLabel} {min} {max} bind:value={() => date, (d) => update(d, time)} />
  </div>
  <div class="w-28">
    <TimePicker label={timeLabel} bind:value={() => time, (t) => update(date, t)} />
  </div>
</div>
