# freelance-crm

Local-first desktop CRM for freelancers: clients, projects, tasks, calendar,
time tracking, invoices and an encrypted vault for passwords and SSH keys.

Built with Tauri 2, Rust and Svelte 5. Licensed under MIT.

- Product requirements: [PRD.ru.md](PRD.ru.md)
- Task plan: [TASKS.ru.md](TASKS.ru.md)

## Development

Requirements: Rust (stable), Node 22, pnpm 10, and the
[Tauri system dependencies](https://tauri.app/start/prerequisites/).

```sh
pnpm install
pnpm tauri dev        # run the app
pnpm verify          # every check CI runs: fmt, clippy, tests, cargo-deny, lint, types
pnpm verify --quick  # without release clippy and cargo-deny
```

`pnpm install` sets up git hooks (lefthook): staged files are format- and
lint-checked on commit, `pnpm verify --quick` runs before push.
`cargo deny` needs [cargo-deny](https://github.com/EmbarkStudios/cargo-deny)
(`cargo install --locked cargo-deny`).

The project is developed test-first: every change starts with a failing test.

## Layout

- `crates/crm-core` — business logic, database, crypto; no Tauri dependency
- `src-tauri` — thin Tauri layer: commands, app state, plugins
- `src` — SvelteKit SPA frontend
