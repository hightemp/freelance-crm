export interface RovingItem {
  disabled?: boolean;
}

/**
 * Index of the next enabled item when moving by `step` (+1 / -1) from
 * `from`, wrapping around. Returns `from` if nothing else is enabled.
 */
export function nextEnabled(items: readonly RovingItem[], from: number, step: 1 | -1): number {
  for (let i = 1; i <= items.length; i++) {
    const index = (from + step * i + items.length) % items.length;
    if (!items[index].disabled) return index;
  }
  return from;
}

export function firstEnabled(items: readonly RovingItem[]): number {
  return items.findIndex((item) => !item.disabled);
}

export function lastEnabled(items: readonly RovingItem[]): number {
  return items.findLastIndex((item) => !item.disabled);
}

/** Maps a navigation key to the target index, or null for other keys. */
export function rovingTarget(
  key: string,
  items: readonly RovingItem[],
  current: number,
): number | null {
  switch (key) {
    case "ArrowRight":
    case "ArrowDown":
      return nextEnabled(items, current, 1);
    case "ArrowLeft":
    case "ArrowUp":
      return nextEnabled(items, current, -1);
    case "Home":
      return firstEnabled(items);
    case "End":
      return lastEnabled(items);
    default:
      return null;
  }
}
