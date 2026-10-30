# TASKS: Freelance CRM

> Russian version: [TASKS.ru.md](TASKS.ru.md)

> Related document: [PRD.md](PRD.md) · Process: TDD (red → green → refactor)

## How to work with this file

- Each task has an ID `T-<milestone>.<n>`, references to PRD requirements (`FR-…`), dependencies, and a **Tests (red)** section — tests are written **before** the implementation and must fail first.
- **Definition of Done** for any task:
  1. the tests from the "Tests" section were written first and are now green;
  2. `cargo clippy -D warnings`, `cargo fmt --check`, `pnpm lint`, `pnpm check` pass;
  3. coverage is not below the threshold (NFR-6);
  4. all new UI strings are present in `messages/en.json` and `messages/ru.json`;
  5. CI is green on Linux, Windows and macOS.
- Statuses: `[ ]` — not started, `[~]` — in progress, `[x]` — done.
- Every milestone ends with a working increment of the application.

---

## M0. Foundation

- [x] **T-0.1 Repository initialization**
  - Run `git init`, add `.gitignore`, `.editorconfig`, `LICENSE` (MIT), `README.md`.
  - Set up the Cargo workspace (`crates/crm-core`, `src-tauri`) and the pnpm workspace.
  - **Tests:** smoke — `cargo test` and `pnpm test` run and pass on an empty test.

- [x] **T-0.2 Tauri 2 + SvelteKit SPA skeleton**
  - Create Tauri 2, SvelteKit (adapter-static, `ssr = false`), Svelte 5, TypeScript strict, Vite.
  - Add Tailwind v4 (no ready-made UI libraries, see FR-UI).
  - **Tests:** Vitest — the `App` component renders; Rust — `crm-core::version()` returns the version from Cargo.

- [x] **T-0.3 Quality tooling**
  - Set up rustfmt, clippy, eslint (svelte and ts), prettier, svelte-check, cargo-deny, cargo-audit.
  - Add pre-commit hooks (lefthook).
  - _Done: rustfmt, clippy (debug and release), eslint, prettier, svelte-check, cargo-deny (also covers the RustSec audit, so a separate cargo-audit is not needed), lefthook, `pnpm verify`._
  - **Tests:** the CI step fails on a deliberately unformatted file (check once and revert).

- [x] **T-0.4 CI (GitHub Actions)**
  - Matrix: ubuntu, windows, macos.
  - Steps: lint → test (Rust and frontend) → coverage (cargo-llvm-cov and vitest v8) with thresholds → build.
  - Cargo and pnpm cache.
  - _Done: `.github/workflows/ci.yml` — lint, cargo-deny, tests and build on 3 OSes, coverage with NFR-6 thresholds (Linux). Not yet run on GitHub: the repository has no remote._
  - **Tests:** the pipeline passes on a branch.

- [x] **T-0.5 Database and migrations** (NFR-2)
  - `Db::open(path)` and `Db::open_in_memory()`; pragmas `WAL`, `foreign_keys=ON`, `busy_timeout`.
  - Migrations via `rusqlite_migration`; `settings` table.
  - **Tests:**
    - migrations apply on an empty database;
    - re-applying is idempotent;
    - `foreign_keys` are enabled;
    - an "all migrations are valid" test (`validate()`).

- [x] **T-0.6 Core domain types**
  - `Id` (UUIDv7), `Timestamp` (UTC ms), `Money { amount_minor: i64, currency: Currency }`, `Currency` (ISO 4217 with the number of decimal places).
  - `CoreError` with codes and parameters.
  - **Tests:**
    - `Money`: adding the same currency; error when adding different currencies; multiplying by hours with banker's rounding;
    - proptest: associativity of addition, no overflow (checked);
    - `Currency::from_str` validates the code;
    - `CoreError` serializes to `{code, params}`.

- [x] **T-0.7 Typed IPC (tauri-specta)**
  - Generate `src/lib/api/bindings.ts` on build (dev); `app_info` command.
  - Frontend wrapper mapping `CoreError` → `AppError`.
  - **Tests:**
    - Rust — a test that the bindings are up to date (generate and compare with the file);
    - Vitest — `mockIPC` returns an error, the wrapper throws an `AppError` with the code.

- [x] **T-0.8 i18n (Paraglide JS 2)** (FR-ST-1, FR-ST-7, NFR-8)
  - Catalogs `messages/en.json` and `messages/ru.json`; locale detected from the system; switching without reload.
  - Utilities `formatDate`, `formatMoney`, `formatDuration` built on `Intl`.
  - Translation of error codes.
  - **Tests:**
    - `en` and `ru` keys match (catalog linter test);
    - `formatMoney(123456, 'RUB', 'ru')` returns `1 234,56 ₽`;
    - `formatMoney(123456, 'USD', 'en')` returns `$1,234.56`;
    - switching the locale re-renders the component;
    - an unknown error code yields a fallback message.

- [x] **T-0.9 Spike: PDF generation (Typst)**
  - Evaluate embedding Typst as a library, Cyrillic and fonts, binary size, render time.
  - Record the decision in ADR `docs/adr/0001-pdf.md`.
  - **Tests:** the prototype renders a PDF with Cyrillic; a test checks that the output starts with `%PDF` and contains the text (via `pdf-extract`).

- [x] **T-0.10 Application shell**
  - Sidebar navigation: Home, Clients, Projects, Tasks, Calendar, Time, Finance, Vault, Settings.
  - Routing, theme (light, dark, system), persisted window size (`window-state`), `single-instance`.
  - **Tests:**
    - clicking navigation links changes the route;
    - the theme switcher sets `data-theme`;
    - accessibility: all navigation items are reachable with Tab.

- [x] **T-0.11 Settings (backend and UI)** (FR-ST-*)
  - `SettingsService` with typed keys and default values; settings screen (language, theme, default currency, first day of week).
  - **Tests:**
    - reading a missing key returns the default;
    - write/read round-trip;
    - an invalid value yields `validation.*`;
    - UI — a language change is saved via IPC (`mockIPC`).

- [x] **T-0.13 UI kit: design tokens and basic elements** (FR-UI-1, 4)
  - CSS variables (colors, spacing, radii, shadows, typography) for the light and dark themes.
  - Components `Button`, `TextField`, `TextArea`, `Checkbox`, `Radio`, `Switch`, `Spinner`, `Progress`, `Tabs`, `Accordion`.
  - Custom scrollbars, selection color, `:focus-visible`.
  - **Tests (Vitest + Testing Library + axe-core):**
    - every component is keyboard-operable (Space and Enter for Checkbox and Switch, arrow keys for the Radio group and Tabs);
    - correct ARIA roles and states (`aria-checked`, `aria-selected`, `aria-invalid`, `aria-describedby` on the error);
    - `disabled` blocks events;
    - axe finds no violations.

- [x] **T-0.14 UI kit: overlays** (FR-UI-2, 6)
  - Portal layer, `@floating-ui/dom`.
  - Components `Dialog`, `ConfirmDialog`, `PromptDialog` (promise API `confirm()` and `prompt()` from `$lib/ui`), `Popover`, `Tooltip`, `DropdownMenu`, `ContextMenu`, `Toast`.
  - **Tests:**
    - focus trap and focus return after closing;
    - Escape and backdrop click close the dialog (configurable);
    - `await confirm()` resolves to `true` or `false`;
    - Tooltip appears on hover and focus with a delay and has `role=tooltip`;
    - menu navigation with arrow keys and typeahead;
    - ContextMenu opens on right click and Shift+F10;
    - Toast hides automatically (fake timers), toast queue.

- [x] **T-0.15 UI kit: value pickers** (FR-UI-1)
  - `Select`, `Combobox` (search, async loading, "create…"), `MultiSelect`/`TagInput`, `NumberField`, `MoneyField`, `Slider`, `ColorPicker`, `FileDropZone` (opens the OS dialog via Tauri).
  - **Tests:**
    - ARIA combobox and listbox: arrow keys, Home and End, typeahead, `aria-activedescendant`;
    - filtering and creating a new value;
    - `NumberField` and `MoneyField` parse localized input (`1 234,56` in RU, `1,234.56` in EN) into minor units without floats;
    - min, max and step;
    - ColorPicker returns a valid hex.

- [x] **T-0.16 UI kit: date and time** (FR-UI-1, FR-ST-2)
  - `DatePicker`, `TimePicker`, `DateTimePicker`, `DateRangePicker`; manual input with a locale-based mask; first day of week from settings.
  - **Tests:**
    - month grid generation (pure function) for different first days of week and a leap February;
    - grid navigation with arrow keys and PageUp/PageDown (ARIA grid pattern);
    - manual input parsing for EN and RU;
    - min and max date;
    - range: the end is not before the start.

- [x] **T-0.17 Disabling browser behaviors** (FR-UI-5)
  - Global key handler and `contextmenu` handler; `novalidate` on forms; UI scale via a setting (CSS `zoom` on the root).
  - **Tests:**
    - Ctrl+R, F5, Ctrl+P and Ctrl+F get `preventDefault`, Ctrl+F opens our own search;
    - the native context menu is suppressed outside input fields, and inside input fields our own menu (cut, copy, paste) is shown;
    - reload is allowed in dev mode.

- [x] **T-0.18 Linter banning native elements** (FR-UI-8)
  - A custom ESLint rule (or a test that scans `.svelte` and `.ts` files).
  - _Done as a scanner test `src/test/native-ui.test.ts` (part of `pnpm test`)._
  - **Tests:**
    - fixtures with `<select>`, `<input type="date">`, `confirm(`, `title=` outside `src/lib/ui` produce an error;
    - inside `src/lib/ui` — allowed;
    - clean code passes.

- [x] **T-0.19 UI kit showcase** (FR-UI-7) — S
  - Route `/dev/ui`, excluded from the production build.
  - **Tests:** the route is absent from the production bundle (build manifest check).

- [x] **T-0.12 E2E infrastructure**
  - WebdriverIO + `tauri-driver`, a CI job on Linux (xvfb) and Windows.
  - _Done differently: `@wdio/tauri-service` with a built-in WebDriver server — works on all three OSes, without `tauri-driver` and the system `WebKitWebDriver`. Only under the `e2e` cargo feature; test data lives in a temporary folder (`FREELANCE_CRM_DATA_DIR`)._
  - **Tests:** e2e — the app starts, the main screen is visible.

---

## M1. Clients, contacts, custom fields, tags, notes

- [x] **T-1.1 Client schema and repository** (FR-CL-1, 2, 5)
  - `clients` migration; `ClientRepo` (create, get, update, list with filters, archive, restore, delete). The `contact_methods` table is in T-1.2.
  - **Tests:**
    - a `person` without a last name and first name — validation error;
    - a `company` without a name — error;
    - `display_name` for a person is "Last First Patronymic", for a company — the short name;
    - list filters by status and `archived`;
    - sorting by name is correct for Cyrillic: a `sort_key` is stored (case-folded, "ё" → "е"), since `NOCASE` only works for ASCII.

- [x] **T-1.2 Contact methods** (FR-CL-3)
  - `contact_methods` migration (owner: client or contact; cascade via trigger); validation of email, phone (5–15 digits, stored as entered), Telegram (`@username`, t.me or phone) and URL (without a scheme — `https://`).
  - **Tests:** table-driven validation tests; several values of the same type; deleting a client deletes its contact methods.

- [x] **T-1.3 Tags** (FR-NT-4)
  - `tags` and `taggings` (polymorphic, cascade from the owner via trigger); `TagService` (CRUD, attach/detach, `set_tags`); client filter — all of the given tags.
  - **Tests:**
    - tag name uniqueness is case-insensitive;
    - attach and detach;
    - deleting a tag deletes its taggings;
    - filtering clients by tag.

- [x] **T-1.4 Custom fields** (FR-NT-3)
  - CRUD, field types, ordering. An empty value is allowed (the field is not filled in yet); numbers are stored with `.`, dates as `YYYY-MM-DD`.
  - **Tests:**
    - validation by type (`email`, `url`, `date`, `number`);
    - reordering persists positions.

- [x] **T-1.5 Notes** (FR-NT-1, 2)
  - Polymorphic notes, pinning (does not change `updated_at`).
  - **Tests:** pinned notes come first, then by `updated_at desc`; cascade deletion together with the owner.

- [x] **T-1.6 Contact persons** (FR-CT-1, 2)
  - CRUD for company contacts; primary contact flag (partial unique index). A company with contacts cannot be turned into a `person` (`client.has_contacts`); tags on contacts are not supported.
  - **Tests:**
    - a contact cannot be added to a `person` client;
    - setting a new primary contact clears the flag on the previous one (in a transaction);
    - cascade deletion.

- [x] **T-1.7 M1 Tauri commands**
  - Commands and DTOs for clients, contacts, contact methods, tags, fields and notes (30 commands); bindings regenerated. The clock is injected via `AppState`.
  - **Tests:** thin command tests via `mock_builder` (happy path and error mapping).

- [x] **T-1.8 UI: client list** (FR-CL-7)
  - Table with search, filters (status, tags, archive) and column sorting; empty state and "nothing found" with a filter reset. `Client` returns `tagIds` in a single subquery (no N+1).
  - **Tests (Vitest):**
    - rendering the list from `mockIPC`;
    - typing in search triggers a debounced request;
    - status filter;
    - empty state.

- [x] **T-1.9 UI: client form**
  - person/company switch; full name or company name fields; contact methods; default currency and rate. Page `/clients/new`; backend errors (with `index`) are shown under the field; after a partial failure, saving again updates rather than creating a duplicate.
  - **Tests:**
    - zod validation (errors in both languages);
    - switching the type shows the right fields;
    - submit calls the command with a correct DTO.

- [x] **T-1.10 UI: client card** (FR-CL-6)
  - Overview and tabs; at this stage Contacts (with contact methods), Notes, Fields and Tags work (new tags are created on the fly), the other tabs are placeholders. Routes `/clients/[id]` and `/clients/[id]/edit`. Field order is changed with up/down buttons (drag and drop — later).
  - **Tests:** switching tabs; adding a contact, note and field from the card.

- [x] **T-1.11 UI: Markdown editor and rendering**
  - CodeMirror 6 and preview; rendering with `marked` + DOMPurify. Links open in the system browser (opener), remote images are stripped, task checkboxes use our own markers instead of native `input`s. Used in notes.
  - **Tests:** `<script>` and `onerror` are stripped (XSS test); GFM tables and checkboxes render.

- [x] **T-1.12 Client deletion and archive** (FR-CL-2, FR-CL-8)
  - Confirmation dialog with counts of related entities and an "Archive instead" button; archive and restore from the card menu. The `DeleteImpact` counters are extended as modules appear (projects, tasks, files…); blocking deletion when invoices exist is in T-9.1, once the invoices table exists.
  - **Tests:**
    - core — `delete_impact(client_id)` returns correct counts;
    - ~~deletion is forbidden when invoices exist (`client.has_invoices`)~~ → moved to T-9.1;
    - UI — counts are shown and archiving is offered.

- [x] **T-1.14 Country and requisites schema registry** (FR-CO-1, 2)
  - Built-in schema files `crates/crm-core/countries/ru.toml` and `intl.toml` (code `INTL`): legal forms, fields, groups, EN and RU labels, tax presets, invoice template key. A legal form's validator can override the field validator (INN (taxpayer ID) of 10 digits for an LLC (OOO), 12 for sole proprietors (IP) and self-employed).
  - Schema loader and validator.
  - **Tests:**
    - all built-in schemas are valid (labels exist in all languages, validator references exist);
    - Russian validators (table-driven tests with valid and invalid values): INN of 10 and 12 digits (checksums), OGRN (primary state registration number) of 13 and OGRNIP of 15 digits, KPP (tax registration reason code) of 9 characters, BIK (bank identifier code) of 9 digits, settlement account with a check against the BIK, correspondent account;
    - generic validators: IBAN (mod 97), SWIFT/BIC (format);
    - the set of required fields depends on the legal form (self-employed has no KPP and OGRN).

- [x] **T-1.15 Requisites: service and UI** (FR-CL-4, FR-ST-4, FR-CO-3)
  - `RequisitesService` (owner: profile or client); a dynamic form built from the schema (country → legal form → fields by group).
  - The contractor profile in settings, a "Requisites" tab in the client card.
  - Saving returns `saved` or `invalid` with all field errors at once. The UI asks to confirm a country change only if some fields would be lost; the backend always requires an explicit flag. Profile logo and signature (FR-ST-4) — after files (M5).
  - **Tests:**
    - core — saving and validating against the schema; changing the country requires confirmation and clears incompatible fields;
    - UI — the form rebuilds when the country and legal form change; validator errors are shown under the field in the current language; INN input mask.

- [x] **T-1.16 US and UK schemas** (FR-CO-5) — S
  - `countries/us.toml` and `gb.toml` (ISO code `GB`). A new `select` field type with options in the schema (US states); the UI kit `Select` got `required` and `error`.
  - **Tests:**
    - US: EIN (format `NN-NNNNNNN`), ABA routing (3-7-1 checksum), state from the lookup list;
    - UK: UTR (10 digits), Company number (8 characters), VAT (GB + 9 digits with mod 97), sort code (`NN-NN-NN`);
    - the schemas pass the common validator from T-1.14.

- [x] **T-1.13 E2E: client**
  - **Tests:** create a company → add a contact → add a note → find it in the list → archive.

---

## M2. Projects

- [x] **T-2.1 Project schema and service** (FR-PR-1, 2, 3, 5)
  - Currency is inherited from the client, then from settings; the rate — from the client. The unused amount (rate for `fixed`, budget for `hourly`) is discarded. Deleting a client cascades to its projects, and `DeleteImpact` counts them.
  - **Tests:**
    - a project without a client — error;
    - currency and rate are inherited from the client if not set;
    - `fixed` requires a budget, `hourly` — a rate;
    - the deadline cannot be earlier than the start date;
    - archive and restore.

- [x] **T-2.2 Commands and UI: project list and form**
  - List (filters by status and client), form, "Projects" tab in the client card.
  - **Tests:** creating a project from the client card pre-fills the client; validation errors are displayed.

- [x] **T-2.3 UI: project card** (FR-PR-4)
  - Overview and tabs: Tasks, Time, Secrets, Files, Notes, Finance (placeholders for now, except Notes); plus a Fields tab (FR-NT-3), archive and delete menu, edit page. Project tags in the UI — together with tasks (M3).
  - **Tests:** overview renders with status, dates and budget.

---

## M3. Tasks

- [x] **T-3.1 Task schema and service** (FR-TS-1, 2)
  - The columns table with four default columns is created here (management — T-3.10); fractional position keys (`domain/position.rs`) are the basis for T-3.3. The deadline is a local date and an optional time (`due_date`, `due_time`), not a UTC timestamp.
  - **Tests:**
    - creating in the inbox (no project or client), with a client only, with a project;
    - when a project is given, `client_id` is taken from the project, and a mismatch is an error;
    - moving to a column of category `done` sets `completed_at`, moving back to another category clears it.

- [x] **T-3.10 Customizable kanban columns** (FR-TS-8)
  - Core and commands are done; the "UI tests" below need the board and were moved to T-3.6. The "overdue"/"today" filters excluding category `done` — in T-3.2.
  - `TaskStatusService`: create, rename, recolor, reorder, delete with moving tasks, WIP limit; seeding of default columns (with `name_key` for localization).
  - **Tests:**
    - seeding creates 4 columns, their names are localized until renamed;
    - deleting a non-empty column without a target is a `status.not_empty` error, with a target the tasks are moved;
    - the last column of category `todo` or `done` cannot be deleted;
    - reorder persists the order;
    - exceeding WIP yields a warning (not an error);
    - the "overdue" and "today" filters exclude category `done`.
  - **UI tests:**
    - adding and renaming a column directly on the board;
    - dragging columns;
    - delete dialog with choosing a column to move tasks into.

- [x] **T-3.2 Filters and queries** (FR-TS-4)
  - Presets "overdue", "today", "week" (today + 6 days) respecting the user's time zone (`jiff`); tasks in `done` columns are not included in presets. Filters: columns, priority, scope (inbox / client with its projects / project), tags, search, hide done; sorting: board, priority, deadline, creation.
  - **Tests:**
    - table-driven preset tests at a fixed `now`, including day boundaries and DST transitions;
    - combining filters;
    - sorting by priority and deadline.

- [x] **T-3.3 Kanban positions** (FR-TS-5)
  - `move_task(id, status, after, before)`: the neighbors must be in the target column and in the right order, otherwise `task.stale_position` (the board is stale).
  - Fractional indexing (string keys) for `board_position`.
  - **Tests:**
    - inserting between two items;
    - at the start and at the end;
    - 1000 consecutive inserts at the same spot yield a correct order (proptest);
    - moving between columns changes the status.

- [x] **T-3.4 Checklists** (FR-TS-3)
  - The task itself returns `checklistDone`/`checklistTotal` (for board cards without extra requests).
  - **Tests:** CRUD, reordering, progress calculation (0 of 0 — `null`, 1 of 3 — 33%).

- [x] **T-3.5 UI: task list**
  - Filters, sorting, inline status change, virtualization (`VirtualList` in the UI kit). Task and checklist commands were added here.
  - **Tests:**
    - changing a filter updates the query;
    - clicking the checkbox sets status `done`;
    - 10,000 rows render with virtualization (no more than N rows in the DOM).

- [x] **T-3.6 UI: kanban** (svelte-dnd-action)
  - Board at `/tasks` ("List / Board" toggle); optimistic moves, on error the board is reloaded from the server; screen reader announcements are localized; the WIP warning is computed from the cards on the board.
  - **Tests:** a simulated dnd calls `move_task(id, status, before, after)`; on an IPC error — optimistic rollback.
  - **Column UI tests (from T-3.10):** adding and renaming a column directly on the board; dragging columns; delete dialog with choosing a column to move tasks into; localized default column names.

- [x] **T-3.7 UI: task card and form** (Markdown description, checklist, links)
  - Routes `/tasks/new` (`?project=`/`?client=`), `/tasks/[id]`, `/tasks/[id]/edit`; "Tasks" tabs in the client and project cards; a shared `TagEditor` (clients, projects, tasks); task notes.
  - **Tests:** adding a checklist item; choosing a project filters the clients.

- [x] **T-3.8 Quick task creation** (FR-TS-7)
  - Global hotkey Ctrl/Cmd+N (Ctrl/Cmd+K is reserved for the command palette, T-10.2) and a modal; the task goes to the inbox.
  - **Tests:** the hotkey opens the modal; Enter creates a task in the inbox.

- [x] **T-3.9 E2E: tasks**
  - Dragging uses a real pointer (WebDriver actions); the move is also checked after a page reload.
  - **Tests:** create a task in a project → drag it to `in_progress` on the kanban → check a checklist item → close the task.

---

## M4. Vault (secrets)

- [x] **T-4.1 Crypto primitives** (PRD §6.4)
  - RustCrypto: `argon2` 0.6, `chacha20poly1305` 0.11 (XChaCha20-Poly1305), `zeroize`, randomness — `getrandom`. Keys (`Key`) are zeroed on drop and are not printed in `Debug`.
  - `derive_kek(password, salt, params)`, `wrap` and `unwrap` of the DEK, `seal` and `open` (XChaCha20-Poly1305 with AAD).
  - **Tests:**
    - round-trip;
    - a wrong password yields `vault.wrong_password`;
    - modified ciphertext, nonce or AAD — error;
    - the nonce is unique (10,000 calls);
    - Argon2id test vectors from RFC 9106;
    - proptest round-trip on random data;
    - key types implement `Zeroize` (type-level check).

- [x] **T-4.2 Vault state** (FR-VA-1, 2, 10)
  - `Vault` (in `AppState`) keeps the DEK in memory only in the `unlocked` state; `vault_meta` stores the KDF parameters, salt and wrapped DEK. The master password is at least 8 characters. Commands — together with the UI (T-4.8).
  - `VaultService`: `init`, `unlock`, `lock`, `change_password`, `status`.
  - **Tests:**
    - state machine (`uninitialized` → `unlocked` → `locked` → `unlocked`);
    - `init` with an existing vault — error;
    - after `change_password` the old password does not work, the new one does, previously encrypted secrets are readable;
    - secret operations in `locked` yield `vault.locked`.

- [x] **T-4.3 Auto-lock** (FR-VA-3)
  - A background ticker every 15 s; activity from the frontend — at most once per 30 s (`vault_touch`); event `vault://locked`. OS sleep is detected from a gap between ticks (> 2 min) — the same on all OSes, without platform hooks. Setting `vaultAutolockMinutes` (0 — never; the UI field — in T-4.8).
  - Inactivity timer (the clock is injected via the `Clock` trait); activity events come from the frontend; lock on sleep or screen lock (best effort per platform).
  - **Tests:**
    - with a fake clock the lock happens exactly after the timeout;
    - activity resets the timer;
    - a timeout of 0 means "never".

- [x] **T-4.4 Secret model and type templates** (FR-VA-4, 5)
  - A single JSON payload is encrypted (template fields + arbitrary pairs), AAD `secret:<id>:<kind>:<key_version>`. A project secret carries the project's client. The "title in FTS, values not" test was moved to T-10.1 (no index yet).
  - `SecretKind` enum and the field schema of each template (whether a field is required, secret, multiline); the payload is stored as JSON and encrypted.
  - **Tests:**
    - each template validates required fields;
    - no plaintext in the database: the test writes a secret with the marker `PLAINTEXT_MARKER_42` and scans the bytes of the database file and the WAL;
    - the secret title goes into FTS, field values do not.

- [x] **T-4.5 Secrets service and commands**
  - A field is addressed by `FieldRef` (a template field by key or an arbitrary pair by index); `secret_payload` returns all values for the edit form. Clipboard — `tauri-plugin-clipboard-manager` behind the `Clipboard` trait (a recording mock in tests).
  - Commands: list (without values), `reveal_field(id, field)`, create, update, delete, `copy_to_clipboard(id, field)`.
  - The value is copied to the clipboard on the backend, without passing it to the frontend.
  - **Tests:**
    - list contains no encrypted fields;
    - `reveal` in `locked` yields an error;
    - `copy_to_clipboard` calls the plugin (mock).

- [x] **T-4.6 Password generator** (FR-VA-7)
  - OS randomness with rejection sampling; passphrase — EFF Short Wordlist 1 (1296 words, CC BY 3.0, `crates/crm-core/wordlists`). Entropy — `length × log2(alphabet)`.
  - **Tests:**
    - the length is respected;
    - each selected character set appears at least once;
    - excluded look-alike characters do not appear (proptest);
    - entropy calculation;
    - the passphrase uses the wordlist, and the word count and separator are respected.

- [x] **T-4.7 SSH key generator and import** (FR-VA-8, 9)
  - `ssh-key` (RustCrypto); RUSTSEC-2023-0071 (Marvin, `rsa` crate) is in the `deny.toml` exceptions: RSA is only generated and serialized locally, without decryption or signing. Dependencies in the dev profile are built with `opt-level = 3` (RSA-4096 and Argon2 in tests). Import — from pasted text; reading a file via a dialog — in the UI (T-4.9). A "Save private key to file" button (system dialog) — in the SSH secret card.
  - **Tests:**
    - Ed25519 and RSA-4096 are generated, the public key matches the private one;
    - a passphrase-protected key decrypts only with it;
    - importing an OpenSSH key detects the type and comment;
    - export on Unix creates a file with `0600` permissions;
    - a corrupted key yields `ssh.invalid_key`.

- [x] **T-4.8 UI: Vault lock screen and onboarding**
  - Screen `/vault`, an indicator in the sidebar footer (clicking it locks an open vault), the auto-lock timeout in settings. `zxcvbn-ts` is lazy-loaded (only on the creation screen). The warning that recovery is impossible will be replaced by showing the recovery key in T-4.11.
  - Master password creation (strength meter via `zxcvbn-ts`, a warning that recovery is impossible), unlock, manual lock, a state indicator in the header.
  - **Tests:**
    - passwords do not match — error;
    - weak password — warning;
    - a wrong password shows an error and clears the field;
    - the `vault://locked` event hides revealed values.

- [x] **T-4.9 UI: secret list and card**
  - Routes `/vault`, `/vault/new` (`?client=`/`?project=`), `/vault/[id]`, `/vault/[id]/edit` — behind the unlock screen. Regular fields are revealed on open, secret ones — via a button; copying happens on the backend; after locking everything revealed is wiped. Exporting an SSH key to a file via the save dialog — in M5 (together with the dialog plugin).
  - Form based on the type template; masking; "show" and "copy" buttons; generators in the form.
  - A "Secrets" tab in the client and project cards.
  - **Tests:**
    - changing the type rebuilds the fields;
    - "show" calls `reveal_field`;
    - after lock values are hidden;
    - the generator inserts a password into the field.

- [x] **T-4.10 Renewal date → event** (FR-VA-12) — S
  - Done in M6: an all-day event with the secret's title and its links, a reminder one week before on creation (afterwards — as the user configures). The link is `secrets.renewal_event_id`; deleting the secret deletes the event via a trigger.
  - _Moved to M6: there are no events or calendar yet. The renewal date is already stored on the secret (`expires_on`, unencrypted)._
  - **Tests:** when `expires_at` is set, a linked event with a reminder is created or updated; when the date is cleared, the event is deleted.

- [x] **T-4.11 Recovery key** (FR-VA-11; PRD §6.4 item 8)
  - 256 bits + 3 bytes of checksum (first bytes of SHA-256) → 56 base32 characters, 14 groups of 4. The DEK wrapping key — HKDF-SHA256. Saving to a file — system dialog (`tauri-plugin-dialog`), the file is written by the backend and only for a valid key; printing — `window.print()` printing only the key. Password change and key reissue — in settings.
  - RK generation on `init`, base32 encoding in groups with a checksum, `recover(rk, new_password)`, `rotate_recovery_key()`.
  - **Tests (core):**
    - `encode` → `decode` round-trip (proptest);
    - a typo in one character is detected by the checksum (`recovery.checksum`);
    - input is insensitive to case, spaces and hyphens;
    - `recover` with a correct RK unlocks the Vault, sets a new password and keeps secrets readable;
    - a wrong RK yields `recovery.invalid`;
    - after `change_password` the RK remains valid;
    - after `rotate` the old RK is invalid, the new one is valid;
    - the RK is not stored in the database (marker test).
  - **Tests (UI):**
    - the key display screen with "Save to file" and "Print" buttons;
    - the confirmation step asks to enter 2 random groups and does not proceed on a mistake;
    - the "Forgot password?" link on the lock screen leads to a form for entering the RK and a new password.

- [x] **T-4.12 E2E: Vault**
  - Includes recovery key confirmation; copying is verified via the notification (the clipboard is not read in e2e).
  - **Tests:** create a master password → create an SSH secret with generation → lock → the value is unavailable → unlock → copy.

---

## M5. Files and images

- [x] **T-5.1 Content-addressed storage** (FR-FI-2, 6, 7)
  - Temporary files go to `<data>/tmp`, then an atomic `rename`; reference counter in the `blobs` table. Deletion on the last reference — `release`.
  - `BlobStore::put(reader) -> BlobId` (streaming, SHA-256, layout `ab/cd/<hash>`), `get`, `delete_if_unreferenced`.
  - **Tests (tempdir):**
    - identical files are stored once;
    - `ref_count` increases and decreases;
    - at 0 references the file is deleted;
    - exceeding the size limit yields an error, the temporary file is removed;
    - a crash mid-copy leaves no garbage (atomic rename).

- [x] **T-5.2 Encrypted blobs** (FR-FI-5)
  - A custom STREAM construction (`vault/stream.rs`): 19-byte nonce prefix + counter + last-chunk flag; the file key — HKDF(DEK, `file`). For encrypted blobs the plaintext hash is not stored.
  - Chunked STREAM encryption; content addressing is not used for encrypted blobs (random id).
  - **Tests:**
    - round-trip for 0 bytes, 1 byte, exactly 64 KB, 64 KB + 1 byte and 10 MB;
    - reordering or removing a chunk is detected;
    - no plaintext on disk;
    - in `locked` — error.

- [x] **T-5.3 Attachments (polymorphic)** (FR-FI-1)
  - Deleting the owner cascades to attachments via triggers and decrements `ref_count`; files without references are removed by `BlobStore::collect_garbage` (called on startup and after deletions — in T-5.5). `EntityRef` got a `Secret` variant. Attachments of events, invoices and expenses — together with those entities.
  - **Tests:**
    - attach to each owner type;
    - deleting the owner deletes attachments and releases blobs;
    - secret attachments are always encrypted.

- [x] **T-5.4 Image thumbnails** (FR-FI-3)
  - PNG up to 320 px, `image` crate (PNG, JPEG, WebP); small images are not upscaled. The thumbnail is a separate blob (`attachments.thumb_blob_id`); images larger than 64 MB get no thumbnail.
  - **Tests:**
    - PNG, JPEG and WebP produce a thumbnail of at most 320 px preserving the aspect ratio;
    - EXIF orientation is respected;
    - for a non-image there is no thumbnail and no error;
    - for encrypted images the thumbnail is encrypted too.

- [x] **T-5.5 Opening and export** (FR-FI-4)
  - Copies for opening go to `<data>/open/{plain,secure}/<id>/<name>`; `secure` is cleared by a `Vault::on_lock` subscription (manual lock and auto-lock), all of `open` — on startup and exit. "Show in folder" is forbidden for encrypted files (`attachment.encrypted`). On startup, blobs without references are also deleted. File copy commands run off the main thread.
  - Opening with the system application, "show in folder", "save as".
  - Encrypted files are decrypted into a temporary folder that is cleared on lock and exit.
  - **Tests:**
    - the temporary file is created and deleted on `lock()`;
    - cleanup of "orphaned" temporary files on startup.

- [x] **T-5.6 UI: attachments**
  - Drag and drop — Tauri events with file paths; files are accepted by the panel open on screen (they can be dropped anywhere in the window). Paste: on `paste` with an image, the backend reads the picture from the clipboard itself (no bytes passed over IPC). Images are served by the `crmfile://` protocol (`thumb/<id>`, `full/<id>`; encrypted ones — `no-store`, 403 when locked). A "Files" tab/section on clients, projects, tasks and secrets; e2e `files.e2e.ts` checks loading of the thumbnail and the full image.
  - Drag & drop, file picker, pasting an image from the clipboard, list and gallery, lightbox, encryption flag.
  - **Tests:**
    - drop calls the command for each file;
    - the gallery shows only images;
    - the lightbox pages with keys;
    - the encryption flag is unavailable when the Vault is locked (with a hint).

---

## M6. Calendar

- [x] **T-6.1 Event schema and service** (FR-CA-2)
  - Both local values (`start_local`, `end_local`, `tzid`) and UTC instants (`start_at`, `end_at`, half-open interval) are stored; time handling — `jiff`. A time that does not exist due to a DST transition is shifted forward, an ambiguous one takes the earlier offset (as in RFC 5545). All-day events are floating dates (inclusive end). Deleting a client or project deletes events, deleting a task or contact unlinks them; events are counted in `DeleteImpact`.
  - **Tests:**
    - `end` is not before `start`;
    - `all_day` normalizes the time;
    - storing in UTC and `tzid` is correct (round-trip for Europe/Moscow and America/New_York).

- [x] **T-6.2 Recurrence: expansion** (FR-CA-4)
  - Instead of the `rrule` crate (pulls in chrono and a second time zone database) — our own parsing and expansion on `jiff` (`calendar/rrule.rs`): FREQ DAILY…YEARLY, INTERVAL, COUNT, UNTIL, BYDAY with ordinals, BYMONTHDAY, BYMONTH, BYSETPOS, WKST; everything else — `rrule.unsupported`. Occurrences are computed in local time. `series_end_at` limits the query for finite series; no more than 50,000 occurrences per event per call.
  - `expand(event, range) -> Vec<Occurrence>` on `rrule`.
  - **Tests (table-driven):**
    - DAILY, WEEKLY with BYDAY, MONTHLY (the 31st), YEARLY (February 29);
    - COUNT and UNTIL;
    - EXDATE;
    - DST transition: an event at 10:00 local time stays at 10:00;
    - range boundary.

- [x] **T-6.3 Editing series** (FR-CA-4)
  - A modified occurrence is a separate event with `series_id` and `recurrence_id` (the local start of the original occurrence) plus an EXDATE in the series. "This and following" sets the series UNTIL one second before the occurrence (for all-day — the previous day), deletes exceptions and modified occurrences of the cut-off part; if the rule did not change, the remaining COUNT carries over to the new series. From the first occurrence this equals "all".
  - Modes "this only", "this and following" (series split) and "all".
  - **Tests:**
    - "this only" creates an override and an exception;
    - "this and following" sets `UNTIL` on the original series and creates a new series;
    - deleting a single occurrence adds an EXDATE;
    - "all" preserves exceptions.

- [x] **T-6.4 Calendar aggregator** (FR-CA-3)
  - A deadline without a time is a floating day, with a time — an instant in the system time zone. The `show_completed` flag shows completed tasks and finished or cancelled projects. The invoices layer will be connected in M8 (no invoices yet). Sorting: start, then all-day events, then kind and title.
  - `calendar_items(range, layers)` merges events, task and project deadlines and invoice due dates.
  - **Tests:** each layer can be turned on and off; completed tasks are hidden according to the setting; sorting.

- [x] **T-6.5 Reminders: calculation** (FR-CA-5, 6; PRD §6.5)
  - A reminder is "N minutes before" (up to 4 weeks) on an event or task; for all-day events it counts from 09:00 local time. A modified occurrence takes the series' reminders (keyed by RECURRENCE-ID). The check window is `(last_check, now]`, no further than 7 days back, on the first check — 5 minutes; a firing older than 5 minutes is `missed`. Tracking fired reminders (`handled`) — in T-6.6.
  - Pure function `due_reminders(now, last_check, items)`.
  - **Tests:**
    - firing exactly within the window;
    - no repeats after `last_fired_at`;
    - a recurring event yields a reminder for each occurrence;
    - those missed while "the app was closed" are marked `missed`;
    - DST.

- [x] **T-6.6 Reminders: scheduler and notifications**
  - Instead of a tokio task — the same background thread as auto-lock (15 s tick). Fired reminders and the last check time are stored in the database (`reminder_log`, `reminder_state`), so there are no duplicates after a restart. The notification text is the start time (language-neutral: the backend does not know the UI locale). Tray: "Open" and "Quit"; closing the window minimizes to the tray if the tray was created; the e2e build has no tray. The ISC license (libloading) is allowed in `deny.toml`.
  - Background tokio task, `tauri-plugin-notification`, tray.
  - **Tests:** with an advanceable fake clock and a mock notifier, the notification is sent once; after a restart (a new service instance on the same database) there is no duplicate.

- [x] **T-6.7 ICS import** (FR-CA-7)
  - Our own parser (`calendar/ics.rs`): line unfolding, quoted parameters, TEXT escaping, DTEND or DURATION, EXDATE as lists and in UTC, RECURRENCE-ID. Time zones: IANA as is, Windows names (Outlook) — via a mapping table, otherwise X-LIC-LOCATION from VTIMEZONE, otherwise local with a warning. From VALARM, DISPLAY and AUDIO relative to the start are taken. Fixtures — `crates/crm-core/tests/fixtures/ics` (in the style of Google and Apple exports; `*.ics -text` in `.gitattributes`). Re-import finds the event by UID. Preview — the `preview_ics` command, selecting events and linking — parameters of `import_ics`; UI — in T-6.9.
  - **Tests (snapshot fixtures of Google and Apple exports):**
    - VEVENT, RRULE, EXDATE, VALARM → reminders;
    - TZID and VTIMEZONE;
    - all-day;
    - a broken file yields a partial import with an error report;
    - re-importing with the same UID updates the event rather than duplicating it.

- [x] **T-6.8 ICS export** (FR-CA-8)
  - For each TZID a VTIMEZONE is written with real transitions from `jiff` over the event period ±1 year (10 years for infinite series); UTC values — with `Z`, all-day — `VALUE=DATE` with an exclusive DTEND. Modified occurrences — a VEVENT with the series UID and RECURRENCE-ID; reminders — VALARM DISPLAY. Lines are folded at 75 octets without splitting characters. The file is written via a temporary file and a rename.
  - **Tests:**
    - export → import round-trip preserves the data;
    - RFC 5545 validity (folding lines at 75 octets, escaping);
    - filters by client, project and range.

- [x] **T-6.9 UI: calendar** (custom components)
  - Time is shown in the system's local time zone and in the locale's clock format (12/24 h). Multi-day timed events go in the all-day row of the week view. Event form: recurrence via presets or a custom RRULE, end (never, N times, date), reminders, time zone, link to a client and project; for series — a "this only / this and following / all" choice on save and delete. Import dialogs (preview, event selection, linking, problem report) and export dialogs (everything, client, project, period). Vitest runs with `TZ=UTC`. Dragging events (FR-CA-9, priority S) is postponed.
  - Month, week, day and agenda views; layers; click to create; series edit dialog; drag to move (S); localization (first day of week, language).
  - **Tests:**
    - the layout function for overlapping events in a day (columns and widths) — table-driven tests;
    - wrapping multi-day events across weeks in the month view;
    - "+N more" when a cell overflows;
    - switching views;
    - turning off a layer hides its items;
    - clicking a slot opens the form with a pre-filled time;
    - editing a recurring event shows the mode choice.

- [x] **T-6.10 UI: missed reminders on startup** (FR-CA-6)
  - The first reminder check runs on startup before the UI loads; the list is shown in a dialog and updated on the `reminders://missed` event. "Close" hides it until the next missed ones, "Mark as seen" clears it.
  - **Tests:** when there are `missed` reminders, the list is shown; "mark as seen" clears it.

- [x] **T-6.11 E2E: calendar**
  - WebDriver cannot open native file dialogs: export and import are invoked with the same command as the UI. "Clean database" — deleting the whole series before import; the agenda before and after is compared.
  - **Tests:** create a weekly event → modify one occurrence → export ICS → import into a clean database → occurrences match.

---

## M7. Time tracking

- [x] **T-7.1 Time entries and timer** (FR-TT-1, 2, 3)
  - A running timer is an entry without `ended_at` (unique partial index: at most one). The target is a task, project or client; the project and client are derived from the task. Setting `timerStopPrevious` (default — stop the previous one). Deleting a task keeps the time on the project; deleting a client or project deletes the entries (counted in `DeleteImpact`). `invoice_id` has no foreign key yet — invoices come later.
  - **Tests:**
    - starting while a timer is already running stops the previous one (or yields an error — depending on the setting);
    - stopping computes the duration;
    - the timer is restored after a "restart" (a new service on the same database);
    - a manual entry with an overlap yields a warning (not an error);
    - an entry with `invoice_id` cannot be modified.

- [x] **T-7.2 Reports** (FR-TT-4, 6)
  - Rate cascade — project, then client: tasks have no rate of their own in the schema. Money — only from billable entries; a running timer is not included in the report. Rounding up per entry, setting `timeRoundingMinutes` (0, 5, 6, 10, 15, 30, 60). The day is by start date in the system time zone. CSV headers are passed by the UI (in the UI language), CSV rows are in ascending time order.
  - **Tests:**
    - grouping by day (in the user's time zone), project and task;
    - money totals at the task, project or client rate (rate cascade);
    - rounding to 15 minutes (C);
    - CSV export (snapshot, escaping).

- [x] **T-7.3 UI: timer in the header and tray**
  - The header is the top bar of the main area (`TimerWidget`). Tray: a "Stop timer" item (enabled while the timer runs) and a tooltip with the time; updated by the background tick (every 15 s), stopping from the tray sends `timer://changed`. Fixed in the UI kit: overlays opened from inside a popover (the `Select` list) no longer close it.
  - **Tests:** start and stop from the header; ticking time (fake timers); an event from the tray syncs the UI.

- [x] **T-7.4 UI: time log and report**
  - "Time" page: filters (period, client, project, billable), total hours and amounts, tabs "Log" (by day, inline editing, deletion, overlap warning) and "Report" (grouping by day, project, task), CSV export, manual entry creation. Entries from an invoice are shown with a lock, without edit buttons.
  - **Tests:** filters; totals; inline editing; locked entries cannot be edited.

---

## M8. Interaction history

- [x] **T-8.1 Interactions service** (FR-IN-1)
  - The cursor is `(occurred_at, id)`, compared as a tuple; a page holds up to 200 entries. Attachments — via `EntityRef::Interaction`. Deleting a client deletes the history (counted in `DeleteImpact`); deleting a contact or project only unlinks.
  - **Tests:**
    - the contact must belong to the same client;
    - the project must belong to the same client;
    - the feed is sorted by `occurred_at desc` with cursor pagination.

- [x] **T-8.2 Follow-up** (FR-IN-3) — S
  - The link is the `interaction_follow_ups` table (tasks and events get no new fields); an interaction has a list of follow-ups. A quote from the interaction is inserted into the task or event description; the event also gets the contact.
  - **Tests:** creating a task or event from an interaction carries over the client, project and a link to the source.

- [x] **T-8.3 UI: feed on the client card** (FR-IN-2)
  - "History" tab: feed (`role="feed"`), filter by type, add form (type, date and time, contact, project, Markdown), deletion, follow-up as a task (by title) or an event (the calendar event form). Loading more — `IntersectionObserver` and a "Show more" button for keyboard users.
  - **Tests:** filter by type; adding an entry; infinite scroll requests the next page.

---

## M9. Finance

- [x] **T-9.1 Invoices: model and calculation** (FR-FN-1, 2, 3)
  - `draft`, `sent`, `cancelled` are stored; `paid`, `partially_paid`, `overdue` are computed ("today" — in the user's time zone). Quantity is in thousandths; discount — a percentage in basis points or a fixed amount (not more than the subtotal); tax in basis points. The `payments` table is created here (the status depends on it), the API — in T-9.4. A client also cannot be deleted when it has payments without an invoice. Editing and deletion are forbidden after the first payment and for cancelled invoices (`invoice.locked`).
  - **Tests (100% branches):**
    - line item total, discount (% and fixed), tax;
    - half-even rounding in minor units;
    - a currency with 0 decimals (JPY) and with 3 decimals (KWD);
    - mixing line item currencies is forbidden;
    - computed `overdue` status at a fixed `now`;
    - `partially_paid` and `paid` based on the sum of payments.
    - a client with invoices cannot be deleted (`client.has_invoices`), `DeleteImpact` shows invoices (moved from T-1.12).

- [x] **T-9.2 Invoice numbering** (FR-FN-3)
  - The template is the `invoiceNumberTemplate` setting: `{YYYY}`, `{YY}`, `{MM}` and a single counter `{N…}` (width is the number of N's, beyond the width it grows). The counter is the `invoice_counters` table, keyed by "template + period" (month with `{MM}`, otherwise year, otherwise unbounded), incremented with a single `UPSERT … RETURNING` — atomically and across connections. Numbers taken manually are skipped. An empty number on creation means auto-numbering; the preview (`preview_invoice_number`) does not consume the counter.
  - **Tests:**
    - template `INV-{YYYY}-{NNN}`;
    - the counter resets in a new year;
    - uniqueness under concurrent creation (transaction);
    - a manual number is checked for uniqueness.

- [x] **T-9.3 Invoice from time entries** (FR-FN-4, FR-TT-5)
  - A line item is "Project — Task" (or the project, or the client for time without a task) per rate; quantity — hours in thousandths with rounding from settings; unit — "ч"/"h" depending on the invoice language. The invoice is created as a draft in the rates' currency; no rate, different currencies or (single line) different rates are errors. Cancelling (`cancelled`) and deleting a draft release the entries.
  - **Tests:**
    - only billable, not-yet-invoiced entries within the period are taken;
    - grouping by task and as a single line;
    - entries are marked with `invoice_id`;
    - cancelling an invoice (`cancelled`) releases the entries.

- [x] **T-9.4 Payments** (FR-FN-6)
  - Only an issued (`sent`) invoice can be paid, in its currency; the client and project are taken from the invoice. A payment without an invoice requires a project. Overpayment — an `overpaid` warning in the response.
  - **Tests:**
    - partial payment;
    - overpayment yields a warning;
    - deleting a payment recalculates the status;
    - a payment without an invoice counts toward project income.

- [x] **T-9.5 Expenses** (FR-FN-7)
  - Category is free text; suggestions are already used categories, case-insensitive (`category_key`). The receipt is an `EntityRef::Expense` attachment. Deleting a project unlinks expenses.
  - **Tests:** CRUD; categories; attaching a receipt (via M5); project profit equals paid minus expenses (per currency).

- [x] **T-9.6 Invoice PDF** (FR-FN-5, FR-CO-3; following T-0.9)
  - Document model — `finance/document.rs` (template by the contractor's country: RU → `ru_invoice`, otherwise `international`; labels by the invoice language), rendering — `crm_pdf::render_invoice` (Typst, templates in `crates/crm-pdf/templates`, shared `common.typ`). "VAT exempt" (Без НДС): self-employed — NPD (professional income tax), IP/OOO (sole proprietor/LLC) — USN (simplified taxation system) (the OSNO regime with VAT is not in the MVP; explicit regime choice — T-9.12). The requisites snapshot (`invoices.snapshot`) is taken on the transition to `sent` and reset when returning to draft. Profile images — the `profile_images` table (PNG/JPEG up to 2 MB, validated by decoding); setting `invoiceShowSignature`, override — `invoices.show_signature`. The text layer snapshot was recorded from the current layout (pdf-extract joins lines after `\`). The image upload UI — in T-9.8.
  - Templates `international` and `ru_invoice` ("Счёт на оплату" (invoice for payment): bank details, "Без НДС", amount in words); selection by the contractor's country; requisites snapshot in the invoice; logo.
  - **Tests:**
    - amount in words in Russian (rubles and kopecks, declensions: 1 рубль, 2 рубля, 5 рублей; 11–14; zero; millions) — table-driven tests;
    - the "Без НДС" wording for NPD and USN;
    - requisites snapshot: editing the profile after issuing does not change the PDF of an old invoice;
    - the PDF is valid and contains the number, amounts and Cyrillic (text extraction);
    - the invoice language affects the labels;
    - text layer snapshot;
    - a missing logo does not break rendering;
    - signature and stamp are rendered only when the option is enabled; the invoice override takes precedence over the profile setting.

- [x] **T-9.12 Taxes** (FR-FN-10)
  - Regimes — the `tax_regimes` table (unique start date, JSON rules); the effective regime is the latest one with `valid_from` ≤ the payment date; the first matching rule (client type and/or currency). Presets: NPD (individuals 4%, companies and IP 6%), USN "Income" 6%, "Custom"; the "USN income minus expenses" preset is not calculated (its base is not payments) and is not shown in the list. Tax is computed per payment with half-even rounding. The payment date is already a local date (`paid_on`). Disabled — `None` instead of numbers.
  - Tax regimes (presets from country schemas and "Custom"), "rate + condition" rules, tax calculation from payments.
  - **Tests (100% branches):**
    - NPD: a payment from an individual client — 4%, from a company or IP — 6%;
    - USN 6% with an editable rate;
    - "Custom" regime;
    - disabled taxes result in no tax fields in aggregates (not zero);
    - tax is computed by the payment date in the user's time zone;
    - rounding in minor units;
    - multi-currency (tax separately for each currency);
    - changing the regime mid-year does not recalculate retroactively (the regime applies from its date).
  - **UI tests:** "track taxes" toggle; choosing a country preset; rule editor.

- [x] **T-9.7 Dashboard: aggregates** (FR-FN-8)
  - **Tests:**
    - with taxes enabled — tax and net income for the period;
    - income by month (by payment date, in the user's time zone), by client, by currency;
    - unpaid and overdue;
    - an empty period yields zeros, not an error.

- [x] **T-9.8 UI: invoices**
  - List, line item editor, "create from time", PDF preview and saving, marking as paid.
  - **Tests:**
    - adding and removing a line item recalculates the total;
    - the "from time" wizard shows entries and the total;
    - in the `paid` status editing is locked.

- [x] **T-9.9 UI: expenses and dashboard** (layerchart)
  - **Tests:** currency switcher; the chart gets correct series; clicking a client leads to its finances.

- [x] **T-9.10 Payment due date in the calendar** (FR-FN-9) — S
  - **Tests:** the "invoices" layer in the aggregator (T-6.4); a reminder for the payment due date.

- [x] **T-9.11 E2E: finance**
  - **Tests:** log time → create an invoice from time → save the PDF → record a partial payment → status `partially_paid` → the dashboard shows the amount.

---

## M10. Search and home screen

- [x] **T-10.1 FTS5 index** (FR-SR-1, 2)
  - Triggers or service hooks that update the index for all entities.
  - **Tests:**
    - search with Cyrillic and case-insensitivity ("ООО ромашка" finds "ООО Ромашка");
    - prefix search;
    - deleting an entity removes it from the index;
    - secret values are not indexed (marker test; the secret title is indexed — moved from T-4.4);
    - ranking: an exact title match ranks higher;
    - performance: 100,000 records, response under 100 ms (an `#[ignore]` bench in CI nightly).

- [x] **T-10.2 UI: global search and command palette** (FR-SR-1, 4)
  - **Tests:** Ctrl/Cmd+K opens the palette; arrow key navigation; Enter opens the entity; "new task" and "lock Vault" commands.

- [x] **T-10.3 Home screen** (FR-SR-3)
  - **Tests:** widgets receive data (`mockIPC`); empty states; a click leads to the entity.

---

## M11. Backup and restore

- [x] **T-11.1 Backup export** (FR-BK-1, 4)
  - `VACUUM INTO` → zip (database, blobs, `manifest.json` with the schema version and hashes) → age (passphrase).
  - **Tests:**
    - the archive decrypts only with the correct password;
    - the manifest is correct;
    - secrets inside remain encrypted (marker test);
    - a backup taken while the database is being written to is consistent.

- [x] **T-11.2 Restore** (FR-BK-3)
  - **Tests:**
    - round-trip: a full data set (fixture generator) → backup → restore into an empty folder → data is identical (dump comparison);
    - a corrupted archive or a wrong password does not change the current data;
    - a backup from a newer schema version yields an error;
    - a backup from an older version is migrated;
    - an automatic backup is created before replacement.

- [x] **T-11.3 Scheduled auto-backup and rotation** (FR-BK-2)
  - The password is stored in the OS keychain (`keyring` crate), or the Vault key is used.
  - **Tests:**
    - the scheduler with a fake clock makes backups on schedule;
    - rotation keeps the last N;
    - an unavailable folder yields an error notification without crashing.

- [x] **T-11.4 JSON and CSV export** (FR-BK-5) — S
  - **Tests:** snapshot of the JSON structure; CSV for clients, projects, tasks and time; secrets are not exported.

- [x] **T-11.5 UI: backup**
  - **Tests:** export wizard (choosing a path, password ×2); restore wizard with a warning; auto-backup settings are saved.

---

## M12. Polish and release

- [x] **T-12.1 Tray and autostart** (FR-ST-6)
  - **Tests:** with the option enabled, closing the window hides it instead of quitting the app; tray menu (timer, Vault lock, quit).

- [x] **T-12.2 Accessibility** (NFR-7)
  - **Tests:** `axe-core` in Vitest for key screens with no violations; e2e keyboard-only navigation through the main scenario.
  - _The keyboard-only scenario is tested in Vitest (user-event): the embedded WebDriver sends untrusted key events, on which the webview takes no default action (Tab, Enter)._

- [x] **T-12.3 Security** (NFR-3)
  - Review of capabilities and CSP; `fs` scope.
  - **Tests:**
    - the CSP header in the config has no `unsafe-eval`;
    - a test that the frontend cannot read a file outside the allowed scope;
    - logs do not contain a secret marker.

- [x] **T-12.4 Performance** (NFR-1)
  - **Tests:** benchmarks (`criterion`) for lists, search and aggregates on synthetic data; the startup budget is checked by an e2e measurement.

- [x] **T-12.5 Build and signing**
  - Linux (AppImage, deb, rpm), Windows (MSI, NSIS), macOS (dmg, notarization if a certificate is available).
  - Auto-update via `tauri-plugin-updater` (optional, can be disabled).
  - **Tests:** CI artifacts build on all platforms; smoke e2e on the built binary (Linux and Windows).
  - _Done: bundle metadata, the `release.yml` workflow (a draft release on `v*` tags, macOS signing and notarization when the secrets exist), a smoke test of the built binary (`scripts/smoke.mjs`; checked locally on Linux). Auto-update is not enabled: it needs an update signing key (`tauri signer generate`) and a release endpoint, which is for the owner to decide._

- [ ] **T-12.6 Documentation**
  - README (EN and RU), user guide, description of the crypto scheme (`docs/security.md`), a "how to add a language" guide.

- [ ] **T-12.7 Manual macOS QA checklist** — _e2e now works on macOS too (T-0.12); the checklist is only needed for what cannot be automated (signing, notarization, system dialogs)._

---

## Order and dependencies (summary)

```
M0 ─┬─ M1 ── M2 ── M3 ─┬─ M6 ── M7 ── M9 ─┐
    │                  │                  ├─ M10 ── M11 ── M12
    └──── M4 ── M5 ────┴─ M8 ─────────────┘
```

- M4 (Vault) can proceed in parallel with M1–M3 after M0.
- M5 depends on M4 (encrypted files).
- The UI kit (T-0.13–T-0.18) is built before the M1 screens; new components are added to the kit as needed, also via TDD.
- Taxes (T-9.12) depend on the country schemas (T-1.14).
- M9 depends on M7 (invoice from time) and M5 (receipts).
- M10 (FTS) indexes entities from all previous modules; indexing hooks are easier to add right away in each module, and T-10.1 is the finalization.
