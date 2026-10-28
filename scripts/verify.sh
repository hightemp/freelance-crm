#!/usr/bin/env bash
# Full local quality gate: the same checks as CI. Stops at the first failure.
# Usage: pnpm verify [--quick]   (--quick skips release clippy and cargo-deny)
set -euo pipefail
cd "$(dirname "$0")/.."

quick=false
[[ "${1:-}" == "--quick" ]] && quick=true

step() {
  local name=$1
  shift
  printf '\n\033[1m▶ %s\033[0m\n' "$name"
  if ! "$@"; then
    printf '\n\033[31m✖ verify failed at: %s\033[0m\n' "$name" >&2
    exit 1
  fi
}

step "rustfmt" cargo fmt --all --check
step "clippy" cargo clippy --workspace --all-targets --quiet -- -D warnings
$quick || step "clippy (release)" cargo clippy --workspace --release --quiet -- -D warnings
step "cargo test" cargo test --workspace --quiet
# NFR-6: core crates keep >= 85 % line coverage (needs cargo-llvm-cov).
$quick || step "rust coverage" pnpm --silent coverage:rust
$quick || step "cargo deny" cargo deny --log-level error check
step "prettier + eslint" pnpm --silent lint
step "svelte-check" pnpm --silent check
if $quick; then
  step "vitest" pnpm --silent test
else
  # NFR-6: thresholds live in vite.config.ts (test.coverage.thresholds).
  step "vitest + coverage" pnpm --silent coverage
fi

# FR-UI-7: dev-only code (UI kit showcase) must not reach production.
$quick || step "build + leak check" bash -c "pnpm --silent build >/dev/null && pnpm --silent check:build"

printf '\n\033[32m✔ all checks passed\033[0m\n'
