import { baseLocale, isLocale, overwriteGetLocale, type Locale } from "$lib/paraglide/runtime";

export const i18n = $state<{ locale: Locale }>({ locale: baseLocale });

// Message functions call getLocale() while rendering. Reading $state here
// makes every rendered message re-render when the locale changes.
overwriteGetLocale(() => i18n.locale);

/** Picks the first supported locale from the OS/browser language list. */
export function detectLocale(languages: readonly string[]): Locale {
  for (const tag of languages) {
    const base = tag.toLowerCase().split("-")[0];
    if (isLocale(base)) return base;
  }
  return baseLocale;
}

export function setAppLocale(locale: Locale): void {
  i18n.locale = locale;
  if (typeof document !== "undefined") document.documentElement.lang = locale;
}
