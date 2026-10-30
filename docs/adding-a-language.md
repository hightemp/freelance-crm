# Adding a language

The interface is translated with [Paraglide](https://inlang.com/m/gerre34r/library-inlang-paraglideJs):
one JSON catalog of messages per language in `messages/`, compiled into
typed functions (`m.client_new()`) by `pnpm i18n`. English (`en`) is the base
language; Russian (`ru`) is the second one. Below, `de` stands for the new
language's code (ISO 639-1).

## 1. Messages

1. Add the code to `locales` in `project.inlang/settings.json`:
   `"locales": ["en", "ru", "de"]`.
2. Copy `messages/en.json` to `messages/de.json` and translate every value.
   Keep the keys and the `{placeholders}` as they are.
3. Add the catalog to `src/lib/i18n/catalogs.test.ts`, which checks that all
   catalogs define the same keys.
4. Run `pnpm i18n` (or `make i18n`) and `pnpm test`.

The catalogs avoid plural forms: counts are written with abbreviations
(`"{count} min"`) or as "label: {count}" (`"Exported files: {count}"`).
Keep to that in the new language, so that one message fits every number.

## 2. Choosing the language

- `crates/crm-core/src/settings.rs`: add the variant to `enum Locale`
  (serialized in lowercase), then run `pnpm bindings` to update the
  TypeScript types.
- `src/lib/app/SettingsForm.svelte`: add the language to the list of the
  Language setting, with its name written in that language ("Deutsch").

The OS language is picked automatically when it is supported
(`detectLocale` in `src/lib/i18n/locale.svelte.ts`), so nothing else is
needed for that.

## 3. Texts made by the backend

A few texts are produced in Rust, where the interface catalogs are not
available, and so exist in English and Russian only:

- the tray menu (`menu_labels` in `src-tauri/src/tray.rs`);
- the notification of a failed automatic backup (`failure` in
  `src-tauri/src/backups.rs`).

Add the new language there, choosing it from `settings.locale`.

## 4. Documents

Invoices are printed in the invoice's own language, `en` or `ru`
(`crates/crm-core/src/finance/document.rs` and the Typst templates in
`crates/crm-pdf/templates`). A new document language needs its labels
there and the option in the invoice editor
(`src/lib/features/finance/InvoiceEditor.svelte`).

## 5. Check

- `make verify` passes;
- switch the language in Settings and walk through the main screens: long
  words should wrap rather than overflow, and dates, numbers and money use
  the language's formats (they come from `Intl`).
