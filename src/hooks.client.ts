// WebdriverIO bridge for end-to-end tests. VITE_E2E is set only by
// `pnpm e2e`, so regular builds drop this import (`pnpm check:build`).
if (import.meta.env.VITE_E2E) void import("@wdio/tauri-plugin");
