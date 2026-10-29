/** Colors given to tags created on the fly, in rotation. */
export const TAG_PALETTE = [
  "#3b82f6",
  "#ef4444",
  "#22c55e",
  "#f59e0b",
  "#8b5cf6",
  "#ec4899",
  "#14b8a6",
  "#64748b",
] as const;

export function nextTagColor(existing: number): string {
  return TAG_PALETTE[existing % TAG_PALETTE.length];
}
