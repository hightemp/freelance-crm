# Changelog

All notable changes to this project are documented here. The format is
based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the
project follows [Semantic Versioning](https://semver.org/).

## [0.9.0] — 2026-10-30

The first public release: every module of the plan ([TASKS.md](TASKS.md))
is in place. It is a beta — the installers are built by CI for the first
time, and the macOS builds are not signed yet.

### Clients and contacts

- Companies and people with contact persons, contact methods, tags, custom
  fields, Markdown notes and files.
- Requisites with per-country validation: Russia (INN, KPP, OGRN, BIK and
  accounts), the US, the UK and international (IBAN, SWIFT/BIC).
- Interaction history (calls, meetings, e-mails, messages) with follow-up
  tasks and events.
- Archiving, and deletion that shows what goes with a client.

### Projects and tasks

- Projects billed hourly or at a fixed price, with their own currency,
  rate, budget and deadline.
- Tasks with statuses, priorities, deadlines, estimates, checklists and
  tags; a list with filters and a kanban board with configurable columns
  and work-in-progress limits; quick capture with Ctrl/Cmd+N.

### Calendar

- Month, week, day and agenda views; recurring events with time zones and
  per-occurrence edits; task, project and invoice deadlines as layers.
- Reminders as system notifications, also with the window closed; missed
  reminders shown at the next start.
- ICS import and export.

### Time tracking

- A timer in the header and the tray, manual entries, overlap warnings.
- Reports by day, project or task with amounts, optional rounding, CSV
  export.

### Finance

- Invoices with automatic numbering, discounts and tax; invoices from
  tracked time.
- PDF documents (Russian and international templates, in English or
  Russian) with the logo, signature and stamp, and amounts in words.
- Partial payments with computed statuses, expenses with receipts.
- A dashboard of income, expenses, profit and open invoices by currency,
  with a monthly chart; optional tax estimates (NPD, USN "Income" or custom
  rules).

### Vault

- Passwords, API keys, SSH keys, servers, bank cards and custom secrets,
  encrypted with a master password (Argon2id, XChaCha20-Poly1305).
- A recovery key, auto-lock, a password and passphrase generator, SSH key
  generation, import and export, encrypted files.

### Search and home screen

- Full-text search over everything but secret values (SQLite FTS5,
  Cyrillic-aware, by prefix) and a command palette (Ctrl/Cmd+K).
- A home screen with overdue and today's tasks, the week's events, unpaid
  invoices, the running timer and recent clients.

### Backups

- One encrypted `.fcrm-backup` file with all data and files (age).
- Restore with integrity and schema checks and a safety copy of the
  replaced data.
- Scheduled backups with rotation, locked by the vault (opened with the
  master password anywhere) or by a password in the OS keychain.
- Export of the non-secret data as JSON and CSV.

### Application

- English and Russian, light and dark themes, interface scale.
- Closing to the tray and starting with the system, both optional.
- A strict content security policy, no file system or shell access for the
  interface, secrets never logged; no telemetry.
- Installers for Linux (AppImage, deb, rpm), Windows (MSI, NSIS) and macOS
  (dmg, Apple Silicon and Intel).

### Known limitations

- macOS builds are not signed or notarized: open the app the first time
  with right click → Open. Windows builds are not signed either, so
  SmartScreen may warn.
- There is no automatic update yet.

[0.9.0]: https://github.com/hightemp/freelance-crm/releases/tag/v0.9.0
