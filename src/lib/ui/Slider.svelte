<script lang="ts">
  let {
    label,
    value = $bindable(0),
    min = 0,
    max = 100,
    step = 1,
  }: { label: string; value?: number; min?: number; max?: number; step?: number } = $props();

  const id = $props.id();
  let dragging = false;

  const snap = (n: number) =>
    Math.min(Math.max(Math.round((n - min) / step) * step + min, min), max);
  let percent = $derived(((value - min) / (max - min)) * 100);

  function fromPointer(event: PointerEvent) {
    const rect = (event.currentTarget as HTMLElement).getBoundingClientRect();
    const ratio = (event.clientX - rect.left) / rect.width;
    value = snap(min + ratio * (max - min));
  }

  function onkeydown(event: KeyboardEvent) {
    const big = Math.max(step, (max - min) / 10) * 5;
    const deltas: Record<string, number> = {
      ArrowRight: step,
      ArrowUp: step,
      ArrowLeft: -step,
      ArrowDown: -step,
      PageUp: big,
      PageDown: -big,
    };
    if (event.key in deltas) value = snap(value + deltas[event.key]);
    else if (event.key === "Home") value = min;
    else if (event.key === "End") value = max;
    else return;
    event.preventDefault();
  }
</script>

<div class="flex flex-col gap-1">
  <span id="{id}-label" class="text-sm font-medium text-fg">{label}</span>
  <div
    role="presentation"
    class="relative h-5 cursor-pointer touch-none"
    onpointerdown={(e) => {
      dragging = true;
      (e.currentTarget as HTMLElement).setPointerCapture?.(e.pointerId);
      fromPointer(e);
    }}
    onpointermove={(e) => dragging && fromPointer(e)}
    onpointerup={() => (dragging = false)}
  >
    <div class="absolute inset-x-0 top-2 h-1 rounded-full bg-surface"></div>
    <div class="absolute top-2 left-0 h-1 rounded-full bg-accent" style:width="{percent}%"></div>
    <div
      role="slider"
      tabindex="0"
      aria-labelledby="{id}-label"
      aria-valuemin={min}
      aria-valuemax={max}
      aria-valuenow={value}
      {onkeydown}
      class="absolute top-0.5 size-4 -translate-x-1/2 rounded-full border-2 border-accent bg-bg"
      style:left="{percent}%"
    ></div>
  </div>
</div>
