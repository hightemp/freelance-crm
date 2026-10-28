/** `aria-describedby` value for a field with optional hint and error. */
export function describedBy(id: string, hint?: string, error?: string): string | undefined {
  const ids = [hint && `${id}-hint`, error && `${id}-error`].filter(Boolean);
  return ids.length ? ids.join(" ") : undefined;
}

export const inputClass = [
  "w-full rounded-[var(--radius)] border border-border bg-bg px-2.5 text-sm text-fg",
  "placeholder:text-muted focus-visible:border-focus disabled:opacity-50",
  "aria-[invalid=true]:border-danger",
].join(" ");
