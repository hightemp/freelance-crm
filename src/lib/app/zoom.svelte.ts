// UI scale replacing WebView zoom (FR-UI-5). Persisted via settings later.

export const ZOOM_LEVELS = [0.8, 0.9, 1, 1.1, 1.25, 1.5] as const;

export const zoom = $state({ level: 1 as number });

export function setZoom(level: number): void {
  zoom.level = ZOOM_LEVELS.reduce((best, l) =>
    Math.abs(l - level) < Math.abs(best - level) ? l : best,
  );
  if (typeof document !== "undefined") {
    document.documentElement.style.setProperty("zoom", String(zoom.level));
  }
}

/** +1 / -1 moves one level, 0 resets to 100 %. */
export function stepZoom(step: -1 | 0 | 1): void {
  if (step === 0) return setZoom(1);
  const index = ZOOM_LEVELS.indexOf(zoom.level as (typeof ZOOM_LEVELS)[number]);
  const next = Math.min(Math.max(index + step, 0), ZOOM_LEVELS.length - 1);
  setZoom(ZOOM_LEVELS[next]);
}
