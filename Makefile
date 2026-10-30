# Shortcuts for the usual commands; `make help` lists them. Everything is
# also available through pnpm and cargo directly (see README.md).

RESVG ?= resvg
ICON_SVG := docs/brand/icon.svg
ICON_PNG := target/icon-1024.png

.DEFAULT_GOAL := help
.PHONY: help install dev build test test-rust test-web e2e lint format check verify verify-quick \
	coverage bindings i18n icons clean clean-caches

help: ## List the targets
	@grep -E '^[a-z0-9-]+:.*## ' $(MAKEFILE_LIST) | \
		awk 'BEGIN { FS = ":.*## " } { printf "  \033[36m%-14s\033[0m %s\n", $$1, $$2 }'

install: ## Install dependencies and git hooks
	pnpm install

dev: ## Run the app with hot reload
	pnpm tauri dev

build: ## Build release installers for this platform
	pnpm tauri build

test: test-rust test-web ## Run all unit and integration tests

test-rust: ## Rust tests (core, PDF, Tauri commands)
	cargo test --workspace

test-web: ## Frontend tests (Vitest)
	pnpm test

e2e: ## End-to-end tests against the real app (builds with the e2e feature)
	pnpm e2e

lint: ## Formatting and lint checks, Rust and web
	cargo fmt --all -- --check
	cargo clippy --workspace --all-targets -- -D warnings
	pnpm lint

format: ## Format Rust and web sources
	cargo fmt --all
	pnpm format

check: ## Type-check the frontend
	pnpm check

verify: ## Every check CI runs
	pnpm verify

verify-quick: ## The checks run before push
	pnpm verify --quick

coverage: ## Coverage reports, Rust and web
	pnpm coverage:rust
	pnpm coverage

bindings: ## Regenerate the TypeScript bindings of the Tauri commands
	pnpm bindings

i18n: ## Compile the translations
	pnpm i18n

icons: ## Regenerate the app icons from docs/brand/icon.svg (needs resvg)
	@command -v $(RESVG) >/dev/null || { echo "resvg not found: cargo install resvg"; exit 1; }
	@mkdir -p $(dir $(ICON_PNG))
	$(RESVG) -w 1024 -h 1024 $(ICON_SVG) $(ICON_PNG)
	$(RESVG) -w 64 -h 64 $(ICON_SVG) static/favicon.png
	pnpm tauri icon $(ICON_PNG)
	rm -rf src-tauri/icons/android src-tauri/icons/ios

clean-caches: ## Delete Rust incremental and coverage caches (they grow large)
	rm -rf target/debug/incremental target/llvm-cov-target

clean: ## Delete all build output
	cargo clean
	rm -rf build .svelte-kit coverage
