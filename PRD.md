# PRD: Freelance CRM (desktop)

> Russian version: [PRD.ru.md](PRD.ru.md)

> Document version: 0.2 · Date: 2026-10-28 · Status: under review · Product license: MIT · Working title: `freelance-crm`

## 1. Overview

**Freelance CRM** is a local desktop application (Tauri 2) for freelancers that keeps all information about clients and the work done for them in one place: clients and companies, contact persons, projects, tasks, calendar, time tracking, finances, interaction history, notes, files and an **encrypted secrets vault** (passwords, API keys, SSH keys, server credentials, etc.).

Key principles:

- **Local-first.** All data is stored locally, with no cloud and no accounts. The application does not need network access.
- **Secret security.** Secrets are encrypted at the field level with a key derived from the master password. Without the master password they cannot be read, even with access to the files.
- **Everything about a client in one place.** The client card gives access to projects, tasks, events, secrets, files, notes, time, money and history.
- **Multilingual.** English and Russian from the first release; the architecture is ready for adding other languages.
- **Test-driven development.** Every feature starts with a failing test.

## 2. Target audience and scenarios

**User:** a freelancer or self-employed professional (developer, designer, DevOps engineer, marketer) who works with 3 to 50 clients and several projects in parallel, holds many credentials, and currently keeps all of this in notes, spreadsheets, messengers and password managers.

### Key user stories

| # | As a user, I want to… | so that… |
|---|---|---|
| US-1 | create a client (individual or organization) with contacts and requisites | everything about them is in one card |
| US-2 | add several contact persons with roles to a company | I know whom to contact about what |
| US-3 | create a project for a client with a rate, budget and deadline | I understand the scope and timeline of the work |
| US-4 | manage tasks as a list and on a kanban board, with checklists | I can plan my work |
| US-5 | create tasks without a project (inbox) | I can quickly jot down an idea and sort it out later |
| US-6 | see meetings, deadlines and recurring events (domain and hosting renewals) in a calendar | I don't miss anything |
| US-7 | get system notifications before events and deadlines | I can react in time |
| US-8 | import and export events in .ics | I can exchange them with Google or Apple Calendar |
| US-9 | store passwords, SSH keys, tokens and database/server credentials linked to a client or project | I can find credentials quickly and not keep them in plain text |
| US-10 | generate passwords and SSH keys right in the application | I don't need third-party tools |
| US-11 | attach files and images (specifications, contracts, mockups, screenshots) | documents don't get lost |
| US-12 | write notes in Markdown and add custom fields (INN, website, Telegram…) | I can store any additional information |
| US-13 | start a timer on a task and see an hours report | I know how much time was spent and how much to bill |
| US-14 | issue invoices (from hours or line items) and save them as PDF | I get paid quickly |
| US-15 | track payments and expenses per project | I can see profitability and client debts |
| US-16 | see an income dashboard by month and by client | I understand my financial position |
| US-17 | record interaction history (calls, meetings, correspondence) | I remember what was agreed |
| US-18 | search across all data at once | I find what I need quickly |
| US-19 | make an encrypted backup (manually and on a schedule) and restore from it | I don't lose data |
| US-20 | switch the UI language (EN/RU) | I work in the language I prefer |

## 3. Scope

### 3.1 In MVP (v1.0)

1. **Clients** — individuals and organizations, statuses, tags, archive.
2. **Contact persons** of companies — roles and contact methods.
3. **Projects** — statuses, billing type (fixed or hourly), rate, budget, currency, dates.
4. **Tasks** — list with filters, kanban, checklists (sub-items), tasks without a project and tasks linked directly to a client.
5. **Calendar** — events, task and project deadlines, recurrence (RRULE), reminders (system notifications), ICS import and export.
6. **Secrets vault (Vault)** — master password, field encryption, type templates, password and SSH key generator, auto-lock.
7. **Files and images** — attachments for any entity, image previews, optional encryption of individual files.
8. **Notes and custom fields** — Markdown with preview, arbitrary "key — value" fields.
9. **Time tracking** — timer, manual entries, reports.
10. **Finances** — multi-currency (no automatic exchange rates), invoices with PDF, payments, expenses, income dashboard.
11. **Interaction history** — per-client feed.
12. **Global search** (FTS) across all non-secret data.
13. **Backup** — encrypted archive (database and files), manual and scheduled to a chosen folder, restore.
14. **i18n** — EN and RU, light and dark theme.
15. **Platforms** — Linux (AppImage, deb, rpm), Windows (MSI, NSIS), macOS (dmg).
16. **Custom UI kit** — all standard browser controls are replaced with in-house components (§4.15).
17. **Country-specific requisites** — contractor and client profiles, country-specific invoice templates (including Russia: self-employed, IP (sole proprietor), OOO (LLC)).
18. **Taxes (optional)** — configurable tax regimes and tax calculation in the dashboard.
19. **Configurable kanban columns** (task statuses).

### 3.2 Out of MVP (later)

- Synchronization between devices (CRDT or server).
- TOTP (2FA codes) in Vault.
- Automatic clipboard clearing after copying a secret (MVP uses plain copying).
- Separate sets of kanban columns per project (MVP has one shared set).
- Automatic exchange rates.
- Live synchronization with CalDAV and Google Calendar.
- Email and messenger integrations, correspondence import.
- Client portal, multi-user mode.
- Mobile versions.
- Additional languages (the architecture is ready for them).

## 4. Functional requirements

Priorities follow MoSCoW: **M** — Must, **S** — Should, **C** — Could.

### 4.1 Clients (FR-CL)

| ID | Requirement | P |
|---|---|---|
| FR-CL-1 | Client type: `person` (full name: last name, first name, patronymic) or `company` (short and full legal name). The display name is computed. | M |
| FR-CL-2 | Status: `lead`, `active`, `paused`, `former`. Archiving (soft delete) and restoring. | M |
| FR-CL-3 | Contact methods: email, phone, Telegram, WhatsApp, website, other (multiple values of each type). | M |
| FR-CL-4 | A client has a **country** and a **legal form**, which determine the set of requisites (see §4.16). For example, for Russia these are INN (taxpayer ID), KPP (tax registration reason code), OGRN (primary state registration number), address and bank. Requisites are validated, e.g. the INN checksum. | M |
| FR-CL-5 | Default currency and hourly rate, inherited by new projects. | M |
| FR-CL-6 | Client card with tabs: Overview, Contacts, Projects, Tasks, Events, Secrets, Files, Notes, Time, Finances, History. | M |
| FR-CL-7 | Client list: search, filter by status and tags, sorting. | M |
| FR-CL-8 | A client can be deleted only after confirmation, showing the number of related entities. Cascade behavior is described in §6.3. | M |

### 4.2 Contact persons (FR-CT)

| ID | Requirement | P |
|---|---|---|
| FR-CT-1 | A contact belongs to a company client: full name, position or role, contact methods, note. | M |
| FR-CT-2 | One contact is marked as primary. | S |
| FR-CT-3 | A contact can be specified in an event and in an interaction. | S |

### 4.3 Projects (FR-PR)

| ID | Requirement | P |
|---|---|---|
| FR-PR-1 | A project is linked to a client (required) and has a name, description (Markdown) and status: `lead`, `active`, `paused`, `completed`, `cancelled`. | M |
| FR-PR-2 | Billing type: `fixed` (budget) or `hourly` (rate); a currency is specified. | M |
| FR-PR-3 | Start date and deadline. The deadline is shown in the calendar. | M |
| FR-PR-4 | Project card: tasks, time, secrets, files, notes, finances (budget, hours spent, invoiced, paid, expenses, profit). | M |
| FR-PR-5 | Project archive. | M |

### 4.4 Tasks (FR-TS)

| ID | Requirement | P |
|---|---|---|
| FR-TS-1 | Task: title, description (Markdown), status (kanban column, see FR-TS-8), priority (`low`, `normal`, `high`, `urgent`), deadline, estimate in hours, tags. | M |
| FR-TS-2 | Linking: to a project, to a client only, or to nothing (inbox). | M |
| FR-TS-3 | Checklist inside a task: items with checkboxes, reordering, progress in percent. | M |
| FR-TS-4 | Task list with filters (status, priority, project, client, tag, "overdue", "today", "this week") and sorting. | M |
| FR-TS-5 | Kanban board by status with drag & drop; the order within a column is preserved. | M |
| FR-TS-6 | The task deadline is shown in the calendar; a reminder can be set for it. | M |
| FR-TS-7 | Quick task creation with a hotkey from any screen. | S |
| FR-TS-8 | **Configurable columns (statuses):** add, rename, set color, reorder, delete. Each column has a **category** (`todo`, `in_progress`, `done`) that drives the logic: in `done`, `completed_at` is set, the task is excluded from "overdue", etc. A column cannot be deleted while it contains tasks — the user is first offered to move them to another column. At least one column of category `todo` and one of category `done` must remain. The default columns are "To do", "In progress", "In review" and "Done"; their names are localized until the user renames them. An optional per-column task limit (WIP) is available. | M |

### 4.5 Calendar (FR-CA)

| ID | Requirement | P |
|---|---|---|
| FR-CA-1 | "Month", "week", "day" and "agenda" (list) views. | M |
| FR-CA-2 | Event: title, description, start and end (or "all day"), time zone (IANA), location or link, links to a client, project, task and contact. | M |
| FR-CA-3 | The calendar shows (each as a layer that can be turned off) task deadlines, project deadlines and invoice due dates. | M |
| FR-CA-4 | Recurrence per RFC 5545 RRULE: daily, weekly (by weekday), monthly, yearly, with an interval, until a date or N times. Exceptions (EXDATE) and "this only / this and following / all" editing are supported. | M |
| FR-CA-5 | Reminders: one or more per event or deadline (N minutes, hours or days before). They are delivered as OS system notifications while the application is running (including when minimized to the tray). | M |
| FR-CA-6 | Missed reminders (while the application was closed) are shown on the next launch. | S |
| FR-CA-7 | .ics import (VEVENT with RRULE, EXDATE and VALARM) with a preview and a choice of what to link the events to. | M |
| FR-CA-8 | Export to .ics: all events, a selected client or project, or a date range. | M |
| FR-CA-9 | Dragging an event to change its time. | S |

### 4.6 Secrets vault — Vault (FR-VA)

| ID | Requirement | P |
|---|---|---|
| FR-VA-1 | On first use, the user creates a master password (with confirmation and a strength indicator). Without it the CRM works, but secrets are unavailable. | M |
| FR-VA-2 | States: `uninitialized`, `locked`, `unlocked`. Unlocking is done by entering the master password. | M |
| FR-VA-3 | Auto-lock after N minutes of inactivity (configurable, 15 by default) and when the OS locks or sleeps (where the OS supports it). Manual lock is available. | M |
| FR-VA-4 | A secret is linked to a client and/or project (or is shared). Only the title, type, links, tags and modification time are stored in plain text; all other fields are encrypted. | M |
| FR-VA-5 | Type templates: **Login/password** (url, login, password), **SSH key** (private key, public key, passphrase, host, user, port), **API token** (service, token, scope), **Database** (type, host, port, db, user, password, connection string), **Server** (host, IP, user, password, panel URL), **FTP/SFTP**, **Domain/hosting** (registrar, login, password, renewal date), **.env** (multiline text), **Bank card**, **Custom** (a set of "key — value" fields). Each field is marked as secret (hidden by default) or regular. | M |
| FR-VA-6 | Show or hide a value and copy it to the clipboard. | M |
| FR-VA-7 | Password generator: length and character sets (Latin letters, digits, special characters, excluding look-alike characters), passphrase mode. An entropy estimate is shown. | M |
| FR-VA-8 | SSH key generator: Ed25519 (default) and RSA 4096, optional passphrase, comment. The result is saved as a secret of type "SSH key". The private key can be exported to a file in OpenSSH format with 0600 permissions. | M |
| FR-VA-9 | Import of an existing SSH key from a file, with type detection and public key extraction. | S |
| FR-VA-10 | Changing the master password without re-encrypting all data (a data-key scheme is used, see §6.4). | M |
| FR-VA-11 | **Recovery key.** When the Vault is created, a random 256-bit key is generated. It is shown **once** as groups of characters (base32, `XXXX-XXXX-…`, with a checksum against typos). It can be saved to a file or PDF, or printed. The user confirms that they have saved the key by entering several of its groups. If the master password is lost, the recovery key unlocks the Vault and lets the user set a new password. The key can be reissued from the settings (while the Vault is unlocked); the old one is then invalidated. The application does not store the key. If both the password and the key are lost, secrets cannot be recovered — the user is explicitly warned about this. | M |
| FR-VA-12 | A renewal or expiration date on a secret (domain, certificate, hosting) can create a calendar event. | S |

### 4.7 Files and images (FR-FI)

| ID | Requirement | P |
|---|---|---|
| FR-FI-1 | Files are attached to a client, contact, project, task, event, secret, interaction, invoice or expense. Sources: file picker dialog, drag & drop, pasting an image from the clipboard. | M |
| FR-FI-2 | Files are copied into the application storage (content-addressed by SHA-256, identical files are not duplicated). The database stores metadata: name, MIME type, size, hash, owner, time. | M |
| FR-FI-3 | Previews (thumbnails) are generated for images; there is a gallery and a full-size viewer. | M |
| FR-FI-4 | Open a file with the system application, "show in folder", "save as…". | M |
| FR-FI-5 | "Encrypt file" flag: the file is stored encrypted with the Vault key and can be opened only while the Vault is unlocked (it is decrypted to a temporary file that is deleted on lock or exit). Attachments of secrets are always encrypted. | M |
| FR-FI-6 | When an attachment is deleted, the file is physically removed if nothing else references it (garbage collection). | M |
| FR-FI-7 | The per-file size limit is configurable (500 MB by default); files are copied in a streaming fashion. | S |

### 4.8 Notes, custom fields, tags (FR-NT)

| ID | Requirement | P |
|---|---|---|
| FR-NT-1 | Multiple notes on any main entity; Markdown editor with preview (GFM, HTML sanitization). | M |
| FR-NT-2 | Pinning a note. | S |
| FR-NT-3 | Custom fields on clients, contacts and projects: key, value and type (`text`, `url`, `email`, `phone`, `date`, `number`); the order is set by dragging. | M |
| FR-NT-4 | Global colored tags for clients, projects, tasks and secrets. | M |

### 4.9 Time tracking (FR-TT)

| ID | Requirement | P |
|---|---|---|
| FR-TT-1 | Start/stop timer on a task or project; only one timer is active at a time; the timer is visible in the header and in the tray. | M |
| FR-TT-2 | The timer survives an application restart: the start time is stored in the database. | M |
| FR-TT-3 | Manual creation and editing of entries: date, start and end or duration, description, billable flag. | M |
| FR-TT-4 | Report: filter by period, client, project and billable; grouping by day, project or task; total hours and amount at the rate. Export to CSV. | M |
| FR-TT-5 | Entries included in an invoice are marked and locked against changes. | M |
| FR-TT-6 | Rounding when billing (e.g. to 15 minutes) is configurable. | C |

### 4.10 Finances (FR-FN)

| ID | Requirement | P |
|---|---|---|
| FR-FN-1 | All amounts are stored in minor units (integer) with an ISO 4217 currency code. Floats are not used. | M |
| FR-FN-2 | The currency is set at the client, project and invoice level. Totals are shown separately per currency; there is no automatic conversion. | M |
| FR-FN-3 | Invoice: number (auto-numbering by a configurable pattern, e.g. `INV-{YYYY}-{NNN}`), client, project (optional), issue and due dates, line items (description, quantity, price, unit), discount, tax (%), note. Statuses: `draft`, `sent`, `paid`, `partially_paid`, `overdue` (computed), `cancelled`. | M |
| FR-FN-4 | Creating an invoice from unbilled billable time entries for a period (grouped by task or as a single line). | M |
| FR-FN-5 | Invoice PDF generation using the **contractor's country template**, with contractor and client requisites, in the invoice language (EN/RU, Cyrillic). MVP has two templates: **international invoice** and **Russian "Счёт на оплату"** (payment invoice) — with bank details, "Без НДС" ("VAT exempt") for self-employed and USN taxpayers, and the amount in words. Signature and stamp are **optional**: the images are uploaded to the contractor profile, their display is enabled in the settings and can be overridden per invoice. | M |
| FR-FN-6 | Payments: date, amount, method, link to an invoice (partial payment is allowed) or to a project without an invoice. | M |
| FR-FN-7 | Expenses: project (optional), category, amount, currency, date, description, receipt attachment. | M |
| FR-FN-8 | Dashboard: income by month (chart), income by client, unpaid and overdue invoices, expenses and profit for the period, and, when taxes are enabled, tax and net income (FR-FN-10). Everything is broken down by currency. | M |
| FR-FN-9 | The invoice due date is shown in the calendar; a reminder can be set for it. | S |
| FR-FN-10 | **Taxes (optional, off by default).** The user enables tax tracking and selects a tax regime from the country presets or creates their own. A regime is a set of "rate + condition" rules: the condition can be the client type (individual or organization) and the currency. Russian presets: **NPD** (professional income tax for the self-employed; 4% from individuals, 6% from legal entities and IPs), **USN "Income"** (simplified tax system, 6%, editable rate). There is a "Custom" regime with an arbitrary rate. Tax is calculated on **received payments**. The dashboard shows the accrued tax for the period and the "net income" per currency. This is an estimate, not an accounting calculation: the UI carries a corresponding notice. | M |

### 4.11 Interaction history (FR-IN)

| ID | Requirement | P |
|---|---|---|
| FR-IN-1 | Entry: client (required), contact and project (optional), type (`call`, `meeting`, `email`, `message`, `other`), date and time, summary (Markdown), attachments. | M |
| FR-IN-2 | Chronological feed on the client card with a filter by type. | M |
| FR-IN-3 | A task or event (follow-up) can be created from an interaction. | S |

### 4.12 Search and navigation (FR-SR)

| ID | Requirement | P |
|---|---|---|
| FR-SR-1 | Global search (Ctrl/Cmd+K) across clients, contacts, projects, tasks, events, notes, interactions, custom fields, file names and secret **titles**. Encrypted values are not indexed. | M |
| FR-SR-2 | Full-text search via SQLite FTS5 with Cyrillic support (`unicode61` tokenizer with `remove_diacritics`), prefix search. | M |
| FR-SR-3 | Home screen: today's and overdue tasks, upcoming events, active timer, unpaid invoices, recent clients. | M |
| FR-SR-4 | Command palette: quick actions ("new task", "start timer", "lock Vault"). | S |

### 4.13 Backup and restore (FR-BK)

| ID | Requirement | P |
|---|---|---|
| FR-BK-1 | Manual export to a single `.fcrm-backup` file: a consistent database snapshot (`VACUUM INTO`) and all files. The archive is encrypted with a backup password (age format with passphrase). | M |
| FR-BK-2 | Scheduled automatic backup (daily or weekly) to a chosen folder, with rotation of the last N copies. The backup password is stored in the OS keychain if the user allows it; otherwise the automatic backup is encrypted with the Vault key. | M |
| FR-BK-3 | Restore: integrity and schema version check, then replacement of the current data. A backup of the current state is made automatically before the replacement. | M |
| FR-BK-4 | Encrypted secrets remain encrypted inside the backup as well: accessing them after a restore requires the master password. | M |
| FR-BK-5 | Export of non-secret data to JSON and CSV (portability). | S |

### 4.14 Settings and i18n (FR-ST)

| ID | Requirement | P |
|---|---|---|
| FR-ST-1 | Language EN or RU, taken from the system by default. Switching takes effect without a restart. | M |
| FR-ST-2 | Locale-aware formatting of dates, numbers and currencies via `Intl`. The first day of the week is configurable. | M |
| FR-ST-3 | Theme: light, dark or system. | M |
| FR-ST-4 | Contractor profile for invoices: country, legal form (for Russia: self-employed, IP, OOO), name or company name, requisites according to the country schema (§4.16), logo, signature. | M |
| FR-ST-5 | Settings for Vault (auto-lock timeout), backup (folder, schedule, rotation), time tracking (rounding), invoice numbering, default currency. | M |
| FR-ST-6 | Minimize to tray on close (on/off), launch at OS startup (optional). | S |
| FR-ST-7 | All UI strings live in message catalogs. Backend error messages arrive as codes and are translated on the frontend. | M |

### 4.15 Custom UI kit (FR-UI)

All standard browser controls are replaced with in-house components in a single style, with theme and i18n support. The components live in `src/lib/ui` and are built on design tokens (CSS variables). Third-party ready-made UI libraries with their own look and feel are not used.

| ID | Requirement | P |
|---|---|---|
| FR-UI-1 | Custom components instead of native ones: `Button`, `TextField`, `TextArea`, `NumberField` (no native spinners, with step and localized input), `MoneyField`, `Checkbox`, `Radio`, `Switch`, `Select`, `Combobox` (search and value creation; replaces `datalist` and autocomplete), `MultiSelect`/`TagInput`, `DatePicker`, `TimePicker`, `DateTimePicker`, `DateRangePicker` (localized, with a configurable first day of the week), `ColorPicker`, `Slider`, `FileDropZone` (instead of `input[type=file]`), `Tabs`, `Accordion` (instead of `details`), `Progress`, `Spinner`. | M |
| FR-UI-2 | Custom overlays: `Dialog`/`Modal`, `ConfirmDialog` and `PromptDialog` (instead of `alert`, `confirm` and `prompt`), `Popover`, `Tooltip` (instead of the `title` attribute), `DropdownMenu`, `ContextMenu` (instead of the WebView context menu), `Toast` (in-app notifications), `CommandPalette`. | M |
| FR-UI-3 | Custom form validation: error messages are shown below the field and localized; native validation popups are disabled (`novalidate`). | M |
| FR-UI-4 | Custom scrollbars, text selection color and focus ring (`:focus-visible`) matching the theme. | M |
| FR-UI-5 | Browser behaviors disabled in production: the default context menu, reload (F5, Ctrl+R), page printing (Ctrl+P), built-in find in page (Ctrl+F, replaced with in-app search), gesture zoom (UI scale is set in the settings), dragging links and images out of the window, Backspace "back" navigation. | M |
| FR-UI-6 | Accessibility: components implement WAI-ARIA patterns (combobox, listbox, dialog, menu, tabs, grid for DatePicker), are fully keyboard-operable, trap focus in modal windows (focus trap) and restore it after closing. | M |
| FR-UI-7 | A component showcase (dev route `/dev/ui`, not built in production) shows all states: default, hover, focus, disabled, error, loading, in both themes. | S |
| FR-UI-8 | Static check: the linter forbids native elements in feature code (`<select>`, `<input type="date|time|color|range|file|number|checkbox|radio">`, `<details>`, `<dialog>`), calls to `alert`, `confirm` and `prompt`, and the `title` attribute. They may be used only inside `src/lib/ui`. | M |
| FR-UI-9 | Only windows drawn by the OS itself rather than the browser remain native: file open and save dialogs, OS notifications, the tray. We do not build our own file browser. | — |

### 4.16 Countries, requisites, legal forms (FR-CO)

| ID | Requirement | P |
|---|---|---|
| FR-CO-1 | There is a registry of **per-country requisite schemas** as data (JSON or TOML embedded in the application). A schema describes legal forms, fields (key, type, required flag, mask, validator), field groups (main, bank, address), EN and RU labels, tax presets and the invoice template. A new country is added without code changes. | M |
| FR-CO-2 | Countries in MVP: **RU** (forms: self-employed (NPD), IP, OOO, individual; fields: INN, KPP, OGRN/OGRNIP, legal address, settlement account, correspondent account, BIK (bank identification code), bank; checksum validation of INN, OGRN and of the settlement account against the BIK), **Generic/International** (forms: individual, company; fields: Tax ID / VAT ID, address, IBAN, SWIFT/BIC, bank, account number). | M |
| FR-CO-5 | Next countries: **US** (sole proprietor, LLC, corporation; EIN or SSN/ITIN, ABA routing and account number, state-based address) and **UK** (sole trader, Ltd; UTR, Company number, VAT number, sort code and account number), with an invoice template for each country. The roadmap after them: EU countries → Malaysia → other Asian countries. | S |
| FR-CO-3 | Requisites are used in the contractor profile (FR-ST-4), on clients (FR-CL-4) and in invoices (FR-FN-5). An invoice stores a **snapshot** of the requisites at the time of issue, so later edits do not change invoices already issued. | M |
| FR-CO-4 | In addition to the schema fields, the user still has arbitrary custom fields (FR-NT-3). | M |

## 5. Non-functional requirements

| ID | Requirement |
|---|---|
| NFR-1 | **Performance:** cold start no more than 2 s; a list of 10,000 tasks opens in no more than 300 ms (with pagination or virtualization); global search — no more than 100 ms over 100,000 records. |
| NFR-2 | **Reliability:** SQLite in WAL mode, `foreign_keys=ON`; all changes run in transactions; migrations are versioned, and irreversible changes are not made without a backup. |
| NFR-3 | **Security:** CSP without `unsafe-eval`; Tauri capabilities follow the principle of least privilege; keys in memory are zeroed (`zeroize`); decrypted secrets are never logged and never get into FTS; Markdown is sanitized (DOMPurify). |
| NFR-4 | **Privacy:** no telemetry and no network requests. The exception is an optional update check that can be turned off. |
| NFR-5 | **Cross-platform:** Linux, Windows and macOS; CI builds and tests on all three. |
| NFR-6 | **Testability:** Rust core layer coverage at least 85% (lines), frontend at least 70%; crypto and finance logic — 100% of branches. |
| NFR-7 | **Accessibility:** full keyboard navigation, visible focus, aria attributes, WCAG AA contrast. |
| NFR-8 | **Localizability:** adding a new language means only adding a new message catalog, with no code changes. |
| NFR-9 | **Size:** installer no larger than 25 MB (excluding fonts for PDF). |

## 6. Architecture and technical decisions

### 6.1 Stack

| Layer | Technology |
|---|---|
| Shell | **Tauri 2** |
| Backend | **Rust** (edition 2024), Cargo workspace |
| Database | **SQLite** via `rusqlite` (bundled, with FTS5), migrations with `rusqlite_migration` |
| Cryptography | `argon2` (Argon2id), `chacha20poly1305` (XChaCha20-Poly1305), `rand`, `zeroize`, `secrecy` |
| SSH | `ssh-key` (generation and parsing of Ed25519/RSA, OpenSSH format) |
| Calendar | `rrule` (recurrence expansion), `icalendar` (ICS), `chrono` and `chrono-tz` |
| Backup | `age` (passphrase encryption), `zip` |
| PDF | **Typst** as an embedded library, a separate `crm-pdf` crate, Noto Sans fonts embedded in the binary (decision: [ADR 0001](docs/adr/0001-pdf.md)) |
| Images | `image` (thumbnails) |
| IPC types | `tauri-specta` (generation of TS bindings for commands and types) |
| Tauri plugins | `dialog`, `fs` (restricted scope), `notification`, `clipboard-manager`, `opener`, `window-state`, `single-instance`, `autostart`, tray |
| Frontend | **Svelte 5** (runes) + **TypeScript (strict)** + **SvelteKit** (adapter-static, SPA) + Vite |
| UI | **Custom UI kit** (`src/lib/ui`) on Tailwind CSS v4 and design tokens; `lucide-svelte` icons; popover positioning with `@floating-ui/dom`. Ready-made UI libraries (shadcn, bits-ui, etc.) are not used. |
| State and data | Svelte runes stores + `@tanstack/svelte-query` (IPC request cache) |
| Forms and validation | `zod` (+ `sveltekit-superforms` client-side or an in-house helper) |
| Calendar UI | Custom month, week, day and agenda views (part of the UI kit). Layout calculation for overlapping events is a pure TS function covered by tests. |
| Kanban | `svelte-dnd-action` |
| Markdown | CodeMirror 6 (editor), `marked` + `DOMPurify` (rendering) |
| Charts | `layerchart` |
| i18n | **Paraglide JS 2** (inlang) — type-safe messages, tree-shaking |
| Rust tests | `cargo test`, `rstest`, `proptest`, `insta` (snapshot), `tempfile`, `cargo-llvm-cov` |
| Frontend tests | **Vitest** + `@testing-library/svelte` + `jsdom`, `mockIPC` from `@tauri-apps/api/mocks`, `@vitest/coverage-v8` |
| E2E | WebdriverIO 9 + `@wdio/tauri-service` with an embedded WebDriver server (`tauri-plugin-wdio-webdriver`): Linux, Windows and macOS without external drivers. The plugins are enabled only by the `e2e` cargo feature and the `tauri.e2e.conf.json` overlay and do not get into production (verified by a test and `check:build`) |
| Quality | `rustfmt`, `clippy -D warnings`, `eslint`, `prettier`, `svelte-check`, `cargo-deny`, `cargo-audit` |
| Package manager | `pnpm` |
| CI | GitHub Actions (matrix: ubuntu, windows, macos) |

### 6.2 Layers and repository structure

```
freelance-crm/
├── crates/
│   └── crm-core/            # pure business logic, no dependency on Tauri
│       ├── src/
│       │   ├── db/           # connection, migrations, pragma
│       │   ├── domain/       # entities, value objects (Money, Currency, Rrule…)
│       │   ├── repo/         # repositories (SQL)
│       │   ├── service/      # use cases (ClientService, VaultService, …)
│       │   ├── crypto/       # KDF, AEAD, key wrapping
│       │   ├── files/        # content-addressed storage, thumbnails
│       │   ├── calendar/     # recurrence, ics, reminders scheduler
│       │   ├── finance/      # invoices, totals, pdf
│       │   ├── backup/
│       │   ├── countries/    # requisite schemas, validators, tax presets (FR-CO)
│       │   └── error.rs      # CoreError with machine-readable codes
│       └── tests/            # core integration tests
├── src-tauri/               # thin layer: commands, state, plugins, tray, events
├── src/                     # SvelteKit SPA
│   ├── lib/
│   │   ├── api/              # generated bindings + query wrappers
│   │   ├── ui/               # custom UI kit (FR-UI)
│   │   ├── components/       # composite application components
│   │   ├── features/         # clients, projects, tasks, calendar, vault, …
│   │   └── i18n/
│   └── routes/
├── messages/                # en.json, ru.json (Paraglide)
├── e2e/
├── Makefile
├── PRD.md, PRD.ru.md
└── TASKS.md, TASKS.ru.md
```

**Rule:** all logic lives in `crm-core` and is tested without Tauri. Commands in `src-tauri` only convert DTOs and call services; they are covered by thin tests. The frontend talks to the backend only through the generated bindings.

### 6.3 Data model (main tables)

Identifiers are UUIDv7 (TEXT). Time is stored in UTC (INTEGER, unix ms); events additionally store an IANA time zone. Money is `amount_minor INTEGER` and `currency TEXT(3)`. Every table has `created_at` and `updated_at`; archivable tables also have `archived_at`.

- `clients` (id, kind: person|company, last_name, first_name, middle_name, company_name, legal_name, status, default_currency, default_hourly_rate_minor, …)
- `contacts` (id, client_id → clients, last/first/middle_name, position, is_primary)
- `contact_methods` (id, owner_type: client|contact, owner_id, kind, value, label)
- `projects` (id, client_id → clients, name, description_md, status, billing_type, rate_minor, budget_minor, currency, start_date, deadline)
- `task_statuses` (id, name?, name_key?, category: todo|in_progress|done, color, position, wip_limit?)
- `tasks` (id, project_id?, client_id?, title, description_md, status_id → task_statuses, priority, due_at, estimate_minutes, board_position, completed_at?)
- `requisites` (id, owner_type: profile|client, owner_id, country, legal_form, fields JSON) — fields according to the country schema
- `tax_regimes` (id, country?, preset_key?, name, rules JSON, active); `settings.tax_enabled`
- `invoices` additionally stores a snapshot of the requisites (`seller_snapshot` and `buyer_snapshot` as JSON) and `template_key`
- `checklist_items` (id, task_id, text, done, position)
- `events` (id, title, description_md, start_at, end_at, all_day, tzid, rrule?, location, client_id?, project_id?, task_id?, contact_id?)
- `event_exceptions` (event_id, original_start_at, kind: cancelled|modified, override_event_id?)
- `reminders` (id, target_type: event|task|project|invoice|secret, target_id, offset_minutes, last_fired_at)
- `interactions` (id, client_id, contact_id?, project_id?, kind, occurred_at, summary_md)
- `time_entries` (id, task_id?, project_id, started_at, ended_at?, duration_seconds, note, billable, invoice_id?)
- `invoices`, `invoice_items`, `payments`, `expenses`
- `secrets` (id, client_id?, project_id?, kind, title, **ciphertext BLOB, nonce BLOB**, expires_at?, key_version)
- `attachments` (id, owner_type, owner_id, blob_id → blobs, file_name, created_at)
- `blobs` (id, sha256, size, mime, encrypted, storage_path, thumb_path, ref_count)
- `notes` (id, owner_type, owner_id, body_md, pinned)
- `custom_fields` (id, owner_type, owner_id, key, value, field_type, position)
- `tags`, `taggings` (tag_id, owner_type, owner_id)
- `vault_meta` (singleton: kdf params, salt, wrapped_dek, wrap_nonce, verifier, version)
- `settings` (key, value JSON)
- `search_index` (FTS5, external content: entity_type, entity_id, title, body)

**Cascade deletion:**

- client → contacts, projects, the client's tasks, the client's events, interactions, notes, custom fields, attachments, the client's secrets;
- project → tasks, time, the project's secrets, attachments.

Invoices and payments are **not deleted** by cascade: while they exist, deleting the client is forbidden and the user is offered archiving instead.

### 6.4 Vault cryptographic scheme

1. On initialization, a random **DEK** (32 bytes) and **salt** (16 bytes) are generated.
2. **KEK** = Argon2id(master_password, salt, m=64 MiB, t=3, p=1). The parameters are stored in `vault_meta` so they can be raised in the future.
3. `wrapped_dek` = XChaCha20-Poly1305(KEK, nonce, DEK). A wrong password results in an AEAD authentication error (no separate verifier is needed).
4. Each secret is a JSON object with fields, encrypted with XChaCha20-Poly1305(DEK, random 24-byte nonce). `secret.id`, `kind` and `key_version` are used as AAD: this protects against swapping ciphertexts between records.
5. Encrypted files are encrypted in a streaming fashion (STREAM construction, 64 KB chunks) with a key derived from the DEK via HKDF (`info="file"`).
6. Changing the password only re-encrypts `wrapped_dek`.
7. In the `unlocked` state, the DEK is held in the backend process memory (`secrecy::SecretBox`, `zeroize` on lock). The DEK is never passed to the frontend; decrypted values are returned individually on request.
8. The **recovery key** is a second wrap of the same DEK: `recovery_wrapped_dek` = XChaCha20-Poly1305(RK, DEK), where RK is 256 random bits. The RK is shown to the user in base32, in groups of 4 characters, with a checksum (CRC). Since the RK is a full-strength random key, it does not need a KDF (HKDF is sufficient). Recovery: RK → unwrap DEK → the user sets a new master password → only `wrapped_dek` is re-encrypted. On reissue, a new RK is generated and `recovery_wrapped_dek` is overwritten.

### 6.5 Reminders

A background task runs in the backend (tokio interval of 30 s). It expands upcoming occurrences (RRULE) over a 48 h horizon, computes the fire time of each reminder and sends a notification via `tauri-plugin-notification`, recording `last_fired_at` to avoid duplicates. The computation logic is a pure function `due_reminders(now, items) -> Vec<Fire>` in core; it is fully covered by tests, including DST transitions.

### 6.6 Error handling and i18n

`CoreError` contains a code (`client.not_found`, `vault.locked`, `vault.wrong_password`, `validation.required{field}` …) and parameters. The frontend translates the code via Paraglide. Texts in PDF invoices and ICS export are generated on the backend from the embedded `en` and `ru` catalogs (the same set of keys).

## 7. Development process (TDD)

1. **Red:** first, a failing test is written — a unit test for core, a component test for UI, an e2e test for key scenarios.
2. **Green:** the minimal code is written to make the test pass.
3. **Refactor:** the code is improved while the tests stay green.
4. Every task in `TASKS.md` has a "Tests" section — those tests are written before the implementation.
5. Test pyramid:
   - **core (Rust):** unit tests and integration tests on in-memory SQLite and `tempdir`, property tests for crypto, money and RRULE;
   - **commands:** thin tests with `tauri::test::mock_builder`;
   - **frontend:** Vitest and Testing Library with `mockIPC`;
   - **e2e:** 5–10 critical scenarios via WebdriverIO with the embedded WebDriver (all three OSes).
6. CI blocks merging if tests or linters fail or if coverage drops below the threshold.
7. Commits follow Conventional Commits; each task gets its own branch or PR.

## 8. Success metrics

- All Must requirements of the MVP are implemented and covered by tests.
- The user finds a client's server credentials in no more than 3 actions (search → secret → copy).
- Not a single case of a plaintext secret being written to disk: verified by a test that scans the database, backup and logs for a marker string.
- Backup and restore are lossless: verified by a round-trip test with a full data set.

## 9. Risks

| Risk | Mitigation |
|---|---|
| Losing the master password means losing the secrets | Explicit warning, recovery key, backup hint |
| The embedded WebDriver server gets into production | The `e2e` feature is off by default (guarded by a test), the frontend bridge exists only with `VITE_E2E`, `check:build` looks for its traces in the build |
| Typst increases the binary size (+33 MB on disk, +10–14 MB in the installer) | Accepted in ADR 0001; fallback is `genpdf` or `printpdf` for simple templates |
| Complexity of RRULE and "this and following" editing | Rely on the `rrule` crate, exhaustive table-driven tests, splitting the series instead of mutating it |
| Reminders are not delivered while the application is closed | Tray, autostart, showing missed reminders on startup; the limitation is explicitly stated in the UI |
| The custom UI kit is a lot of work (DatePicker, Combobox, calendar) | The kit is built in M0 and extended as needed; ARIA patterns are covered by tests; `/dev/ui` showcase |
| Errors in requisites and taxes | Country schemas are data with validator tests; the UI carries an "estimate, not an accounting calculation" notice |
| Large MVP scope | Incremental milestones; each one is a working increment |

## 10. Decisions made (v0.2)

1. Automatic clipboard clearing — not in MVP.
2. Recovery key — in MVP (FR-VA-11, §6.4 item 8).
3. Requisites and invoice templates follow the contractor's country; Russia is supported in MVP (FR-CO, FR-FN-5).
4. Taxes are optional and configurable (FR-FN-10).
5. Kanban columns are configurable (FR-TS-8).
6. Working title — `freelance-crm`, MIT license.
7. All browser UI elements are replaced with custom components (§4.15).
8. Countries: RU and Generic in MVP, then US and UK (FR-CO-5), then EU → Malaysia → Asia.
9. Signature and stamp on invoices are optional.

## 11. Open questions

1. Besides the shared set of columns, are separate sets per project needed? Currently — after MVP.
