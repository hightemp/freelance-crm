export interface MenuItem {
  id: string;
  label: string;
  disabled?: boolean;
  danger?: boolean;
}

export type MenuEntry = MenuItem | { separator: true };

export function isItem(entry: MenuEntry): entry is MenuItem {
  return !("separator" in entry);
}

/**
 * Typeahead: next enabled item whose label starts with `query`. A single
 * character searches after `from` (cycling through matches); a longer query
 * may stay on the current item.
 */
export function typeaheadMatch(items: readonly MenuItem[], query: string, from: number): number {
  const q = query.toLocaleLowerCase();
  const start = query.length === 1 ? 1 : 0;
  for (let i = start; i < items.length + start; i++) {
    const index = (from + i) % items.length;
    const item = items[index];
    if (!item.disabled && item.label.toLocaleLowerCase().startsWith(q)) return index;
  }
  return -1;
}
