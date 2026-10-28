/** Normalizes `#ABC`, `abc`, `#AABBCC` to lowercase `#aabbcc`, or null. */
export function normalizeHex(input: string): string | null {
  const s = input.trim().replace(/^#/, "").toLowerCase();
  if (/^[0-9a-f]{3}$/.test(s)) return `#${[...s].map((c) => c + c).join("")}`;
  if (/^[0-9a-f]{6}$/.test(s)) return `#${s}`;
  return null;
}

export const PRESET_COLORS = [
  "#3b5bdb",
  "#1c7ed6",
  "#0ca678",
  "#37b24d",
  "#f59f00",
  "#f76707",
  "#e03131",
  "#d6336c",
  "#ae3ec9",
  "#7048e8",
  "#868e96",
  "#212529",
];
