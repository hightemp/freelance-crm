<p align="center">
  <img src="docs/brand/icon.svg" alt="" width="128" height="128">
</p>

<h1 align="center">freelance-crm</h1>

<p align="center">
  A local-first desktop CRM for freelancers: clients, projects, tasks, calendar,
  time tracking, invoices and an encrypted vault, in one app that keeps your
  data on your computer.
</p>

<p align="center"><a href="README.ru.md">Русский</a></p>

<p align="center">
  <a href="https://github.com/hightemp/freelance-crm/actions/workflows/ci.yml"><img alt="CI" src="https://github.com/hightemp/freelance-crm/actions/workflows/ci.yml/badge.svg"></a>
  <a href="LICENSE"><img alt="License: MIT" src="https://img.shields.io/badge/license-MIT-blue.svg"></a>
  <img alt="Platforms: Linux, Windows, macOS" src="https://img.shields.io/badge/platforms-Linux%20%7C%20Windows%20%7C%20macOS-lightgrey.svg">
  <img alt="Tauri 2" src="https://img.shields.io/badge/Tauri-2-24C8DB.svg?logo=tauri&logoColor=white">
  <img alt="Rust" src="https://img.shields.io/badge/Rust-stable-orange.svg?logo=rust">
  <img alt="Svelte 5" src="https://img.shields.io/badge/Svelte-5-FF3E00.svg?logo=svelte&logoColor=white">
</p>

## Features

- **Clients and contacts** — companies and people, contact persons, contact
  methods, custom fields, tags, notes, requisites with per-country validation
  (Russia, US, UK and international), and a history of calls, meetings and
  messages with follow-up tasks and events.
- **Projects and tasks** — hourly, fixed or retainer billing; a kanban board with
  configurable columns, checklists, deadlines and quick capture (Ctrl/Cmd+N).
- **Calendar** — month, week, day and agenda views; recurring events with
  time zones, reminders as system notifications, task, project and invoice
  deadlines as layers; ICS import and export.
- **Time tracking** — a timer in the header and the tray, manual entries,
  reports by day, project or task, rounding and CSV export.
- **Finance** — invoices from tracked time with automatic numbering, PDF
  documents (Russian and international templates, logo, signature and stamp),
  partial payments, expenses with receipts, a dashboard of income, expenses and
  open invoices by currency, and optional tax estimates (NPD, USN or custom).
- **Vault** — passwords, SSH keys and other secrets encrypted with a master
  password (Argon2id, XChaCha20-Poly1305), a recovery key, auto-lock, a
  password generator; files can be stored encrypted too.
- **Search** — global full-text search and a command palette (Ctrl/Cmd+K);
  the home screen shows overdue and today's tasks, upcoming events, unpaid
  invoices and recent clients.
- **Backups** — one encrypted `.fcrm-backup` file with all data and files,
  scheduled backups with rotation, restore with a safety copy of the replaced
  data, and export of the non-secret data as JSON and CSV.
- English and Russian interface, light and dark themes, no telemetry.
- Signed automatic updates from GitHub releases (the check can be turned off).

## Documents

| | English | Russian |
|---|---|---|
| Product requirements | [PRD.md](PRD.md) | [PRD.ru.md](PRD.ru.md) |
| Task plan | [TASKS.md](TASKS.md) | [TASKS.ru.md](TASKS.ru.md) |

- [User guide](docs/user-guide.md) ([in Russian](docs/user-guide.ru.md))
- [Security](docs/security.md): what is encrypted, with which keys
- [Adding a language](docs/adding-a-language.md)
- Architecture decisions: [docs/adr](docs/adr)

## Development

Requirements: Rust (stable), Node 22, pnpm 10 and the
[Tauri system dependencies](https://tauri.app/start/prerequisites/).
`cargo deny` needs [cargo-deny](https://github.com/EmbarkStudios/cargo-deny)
(`cargo install --locked cargo-deny`).

```sh
make install       # dependencies and git hooks
make dev           # run the app with hot reload
make test          # Rust and frontend tests
make verify        # every check CI runs: fmt, clippy, tests, coverage, cargo-deny, lint, types
make e2e           # end-to-end tests against the real app
make build         # release installers for this platform
make help          # all targets
```

The same commands are available through pnpm (`pnpm tauri dev`, `pnpm verify`,
`pnpm verify --quick`, `pnpm e2e`) and cargo.

End-to-end tests use WebdriverIO with an embedded WebDriver server, so no
external driver is needed on Linux, Windows or macOS. They run on a throwaway
data directory (`FREELANCE_CRM_DATA_DIR`), never on your real database.

`make install` sets up git hooks (lefthook): staged files are format- and
lint-checked on commit, `pnpm verify --quick` runs before push.

The project is developed test-first: every change starts with a failing test.

The app icon is drawn in [docs/brand/icon.svg](docs/brand/icon.svg);
`make icons` renders the platform icons from it (needs
[resvg](https://github.com/linebender/resvg)).

## Layout

- `crates/crm-core` — business logic, database, crypto, backups; no Tauri dependency
- `crates/crm-pdf` — invoice documents rendered with Typst
- `src-tauri` — thin Tauri layer: commands, app state, tray, scheduler, plugins
- `src` — SvelteKit SPA frontend (Svelte 5, Tailwind CSS 4)
- `e2e` — end-to-end tests
- `messages` — interface translations

## Data and security

Everything is stored locally in an SQLite database and a folder of files in
the app's data directory. Secrets are encrypted with a key that only your
master password (or the recovery key) unlocks; they stay encrypted inside
backups too. Nothing is sent anywhere.

## License

[MIT](LICENSE)
