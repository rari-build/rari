set windows-shell := ["powershell"]
set shell := ["bash", "-cu"]

# List all available commands
_default:
    just --list -u

# --- Setup commands ---

# Setup the entire project (Rust + Node.js)
setup:
    just check-prerequisites
    # Rust setup
    cargo install cargo-binstall
    cargo binstall cargo-insta -y
    cargo binstall cargo-nextest -y
    cargo binstall cargo-machete -y
    # Node.js setup
    corepack enable
    pnpm install
    @echo "✅ Setup complete!"

# Check if all prerequisites are installed
check-prerequisites:
    @command -v cargo >/dev/null 2>&1 || { echo "❌ Cargo is not installed. Please install Rust from https://rustup.rs/"; exit 1; }
    @command -v node >/dev/null 2>&1 || { echo "❌ Node.js is not installed. Please install Node.js from https://nodejs.org/"; exit 1; }
    @command -v pnpm >/dev/null 2>&1 || { echo "❌ pnpm is not installed. Run 'corepack enable' to install it."; exit 1; }
    @echo "✅ All prerequisites are installed"

# --- Build commands ---

# Build everything (Rust + all Node.js packages)
build: build-rust build-node

# Build Rust crates
build-rust:
    cargo build --workspace

# Build Rust crates in release mode
build-rust-release:
    cargo build --workspace --release

# Build all Node.js packages
build-node: _ensure-node-deps
    pnpm build

# Build only the rari package
build-rari: _ensure-node-deps
    pnpm --filter rari build

# Build only create-rari-app
build-create-rari-app: _ensure-node-deps
    pnpm --filter create-rari-app build

# Build the web package
build-web: _ensure-node-deps
    pnpm --filter @rari/web build

# --- Test commands ---

# Run all tests (Rust + Node.js)
test: test-rust test-node

# Run Rust tests with nextest
test-rust:
    cargo nextest run --workspace

# Run Rust tests with standard test runner
test-rust-standard:
    cargo test --workspace

# Run Rust doc tests
test-rust-doc:
    cargo test --workspace --doc

# Run all Rust tests (nextest + doc tests)
test-rust-all: test-rust test-rust-doc

# Run Node.js tests
test-node: _ensure-node-deps
    pnpm -r run test

# Run tests with coverage
test-coverage:
    cargo nextest run --workspace --all-features
    cargo test --workspace --doc

# Run specific test with nextest
test-filter filter:
    cargo nextest run --workspace {{ filter }}

# --- Lint commands ---

# Run all linters (Rust + Node.js + repo)
lint: lint-rust lint-node

# Lint Rust code
lint-rust: clippy machete
    cargo fmt --all --check
    cargo check --workspace --all-features --all-targets --locked

# Run clippy
clippy:
    cargo clippy --workspace --all-targets -- --deny warnings

# Check for unused Rust dependencies
machete:
    cargo machete

# Lint Node.js code
lint-node: _ensure-node-deps
    pnpm lint
    pnpm knip

# Check for unused dependencies and exports with knip
knip: _ensure-node-deps
    pnpm knip

# --- Fix commands ---

# Fix all fixable issues (Rust + Node.js)
fix: fix-rust fix-node

# Fix Rust formatting and linting issues
fix-rust:
    cargo fmt --all
    cargo fix --allow-dirty --allow-staged

# Fix Node.js linting issues
fix-node: _ensure-node-deps
    pnpm lint:fix

# --- Clean commands ---

# Clean all build artifacts
clean: clean-rust clean-node

# Clean Rust build artifacts
clean-rust:
    cargo clean

# Clean Node.js build artifacts
clean-node:
    pnpm clean

# --- Development commands ---

# Start the web development server
dev: _ensure-node-deps
    pnpm --filter @rari/web dev

# Type check all TypeScript code
typecheck: _ensure-node-deps
    pnpm typecheck

# Watch and run tests on file changes
test-watch:
    cargo watch -x "nextest run --workspace"

# --- Release commands ---

# Generate changelog
changelog:
    git-cliff --output CHANGELOG.md

# Preview unreleased changelog
changelog-preview:
    git-cliff --unreleased

# Show latest changelog entry
changelog-latest:
    git-cliff --latest

# Run release process
release:
    cargo run --release --manifest-path tools/release/Cargo.toml --bin release

# Dry run release process
release-dry:
    cargo run --release --manifest-path tools/release/Cargo.toml --bin release -- --dry-run

# Prepare binaries for release
prepare-binaries:
    cargo run --release --manifest-path tools/prepare-binaries/Cargo.toml --bin prepare-binaries

# --- Utility commands ---

# Ensure pnpm dependencies are installed
_ensure-node-deps:
    #!/usr/bin/env bash
    if [ ! -d "node_modules" ] || [ "pnpm-lock.yaml" -nt "node_modules" ] || [ "package.json" -nt "node_modules" ]; then
        echo "📦 Installing Node.js dependencies..."
        pnpm install
    fi

# Run the rari CLI
run *args:
    cargo run --manifest-path crates/rari/Cargo.toml -- {{ args }}

# Run the rari CLI in release mode
run-release *args:
    cargo run --release --manifest-path crates/rari/Cargo.toml -- {{ args }}

# --- Combined workflow commands ---

# Run all checks (lint + test + typecheck)
check: lint test-rust-all test-node typecheck

# Full CI workflow
ci: check build

# Quick development check (faster than full CI)
quick-check: lint-rust typecheck
    cargo check --workspace
