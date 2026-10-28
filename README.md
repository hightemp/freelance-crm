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
cargo test --workspace && pnpm test   # all tests
pnpm lint && pnpm check               # linters and type check
```

The project is developed test-first: every change starts with a failing test.

## Layout

- `crates/crm-core` — business logic, database, crypto; no Tauri dependency
- `src-tauri` — thin Tauri layer: commands, app state, plugins
- `src` — SvelteKit SPA frontend
