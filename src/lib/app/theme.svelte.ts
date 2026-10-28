export type ThemeMode = "light" | "dark" | "system";

export const theme = $state({ mode: "system" as ThemeMode });

let media: MediaQueryList | null = null;

function apply() {
  const dark = theme.mode === "dark" || (theme.mode === "system" && !!media?.matches);
  document.documentElement.dataset.theme = dark ? "dark" : "light";
}

/** Starts following the OS color scheme (for `system` mode). */
export function initTheme(): void {
  media?.removeEventListener("change", apply);
  media = window.matchMedia("(prefers-color-scheme: dark)");
  media.addEventListener("change", apply);
  apply();
}

export function setTheme(mode: ThemeMode): void {
  theme.mode = mode;
  apply();
}
