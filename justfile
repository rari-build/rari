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
build: bundle-react-esm build-snapshot build-rust build-node

# Generate V8 startup snapshot (required before building rari)
# Rebuilds when snapshot is missing, empty, or stale (older than Cargo.toml or extension sources)
build-snapshot:
    #!/usr/bin/env bash
    SNAPSHOT="crates/rari/snapshots/RARI_SNAPSHOT.bin"
    NEEDS_REBUILD=false
    if [ ! -s "$SNAPSHOT" ]; then
        NEEDS_REBUILD=true
    else
        STALE_SOURCES=$(find Cargo.toml crates/rari/Cargo.toml crates/rari/src/runtime/ext -newer "$SNAPSHOT" \( -name "*.toml" -o -name "*.js" -o -name "*.ts" \) 2>/dev/null | head -1)
        if [ -n "$STALE_SOURCES" ]; then
            NEEDS_REBUILD=true
        fi
    fi
    if [ "$NEEDS_REBUILD" = true ]; then
        echo "Generating V8 snapshot..."
        mkdir -p crates/rari/snapshots
        touch "$SNAPSHOT"
        echo 'pub static RESIDUAL_LAZY_ESM_SOURCES: &[(&str, &str)] = &[];' > crates/rari/snapshots/residual_lazy_sources.rs
        echo 'pub static RESIDUAL_LAZY_JS_SOURCES: &[(&str, &str)] = &[];' >> crates/rari/snapshots/residual_lazy_sources.rs
        cargo run --manifest-path tools/snapshot/Cargo.toml -- crates/rari/snapshots
    else
        echo "V8 snapshot is up to date."
    fi

# Build Rust crates
build-rust: build-snapshot
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

build-addon-dev:
    cargo run --manifest-path tools/prepare_binaries/Cargo.toml -- --addon --dev

# --- Test commands ---

# Run all tests (Rust + Node.js)
test: test-rust test-node

# Snapshot was built with ext-full; runtime tests need the same feature set.
# Run Rust tests with nextest
test-rust:
    cargo nextest run --workspace --features rari/ext-full

# Run Rust tests with standard test runner
test-rust-standard:
    cargo test --workspace --features rari/ext-full

# Run Rust doc tests
test-rust-doc:
    cargo test --workspace --doc --features rari/ext-full

# Run all Rust tests (nextest + doc tests)
test-rust-all: test-rust test-rust-doc

# Run Node.js tests
test-node: _ensure-node-deps
    pnpm test:unit:run

# Run tests with coverage
test-coverage:
    cargo nextest run --workspace --all-features
    cargo test --workspace --doc --all-features

# Run specific test with nextest
test-filter filter:
    cargo nextest run --workspace --features rari/ext-full {{ filter }}

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

# Preview unreleased changelog for a package (stdout only; does not write files).
# Package CHANGELOG.md updates happen in `just release` via git-cliff --prepend
# with a package --tag-pattern so v* / cross-package tags are not mixed in.
# Usage: just changelog
#        just changelog create-rari-app
#        just changelog use-cache
changelog package="rari":
    #!/usr/bin/env bash
    set -euo pipefail
    case "{{package}}" in
      rari)
        git-cliff --unreleased --tag-pattern '^rari@' \
          --include-path 'packages/rari/**' --include-path 'crates/rari/**'
        ;;
      create-rari-app)
        git-cliff --unreleased --tag-pattern '^create-rari-app@' \
          --include-path 'packages/create-rari-app/**'
        ;;
      use-cache|"@rari/use-cache")
        git-cliff --unreleased --tag-pattern '^@rari/use-cache@' \
          --include-path 'packages/use-cache/**' --include-path 'crates/rari_use_cache/**'
        ;;
      *)
        echo "Unknown package: {{package}} (expected rari | create-rari-app | use-cache)" >&2
        exit 1
        ;;
    esac

# Alias for clarity
changelog-preview package="rari":
    just changelog {{package}}

# Show latest tagged changelog section for a package (stdout only)
changelog-latest package="rari":
    #!/usr/bin/env bash
    set -euo pipefail
    case "{{package}}" in
      rari)
        git-cliff --latest --tag-pattern '^rari@' \
          --include-path 'packages/rari/**' --include-path 'crates/rari/**'
        ;;
      create-rari-app)
        git-cliff --latest --tag-pattern '^create-rari-app@' \
          --include-path 'packages/create-rari-app/**'
        ;;
      use-cache|"@rari/use-cache")
        git-cliff --latest --tag-pattern '^@rari/use-cache@' \
          --include-path 'packages/use-cache/**' --include-path 'crates/rari_use_cache/**'
        ;;
      *)
        echo "Unknown package: {{package}} (expected rari | create-rari-app | use-cache)" >&2
        exit 1
        ;;
    esac

# Run release process
# Usage: just release
# With OTP: NPM_OTP=123456 just release
release:
    GITHUB_TOKEN=$(gh auth token) cargo run --release --manifest-path tools/release/Cargo.toml

# Dry run release process
release-dry:
    GITHUB_TOKEN=$(gh auth token) cargo run --release --manifest-path tools/release/Cargo.toml -- --dry-run

# Prepare binaries for current platform (release build)
prepare-binaries:
    cargo run --release --manifest-path tools/prepare_binaries/Cargo.toml

# Prepare binaries for development (fast debug build)
prepare-binaries-dev:
    cargo run --manifest-path tools/prepare_binaries/Cargo.toml -- --dev

# Bundle React CJS into ESM for rari's V8 runtime (emits .js into the rari crate)
bundle-react-esm: _ensure-node-deps
    pnpm --filter @rari/bundle-react-esm bundle

# --- Utility commands ---

# Update Node.js dependencies interactively
taze: _ensure-node-deps
    pnpx taze -r -w

# Update GitHub Actions to latest versions
update-actions:
    pnpx actions-up --yes --include-branches

# Ensure pnpm dependencies are installed
_ensure-node-deps:
    pnpm install --frozen-lockfile

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

# --- CI commands ---

# Create empty snapshot placeholders before generating or compiling against snapshot paths
ci-create-snapshot-placeholder:
    #!/usr/bin/env bash
    set -euo pipefail
    mkdir -p crates/rari/snapshots
    touch crates/rari/snapshots/RARI_SNAPSHOT.bin
    echo 'pub static RESIDUAL_LAZY_ESM_SOURCES: &[(&str, &str)] = &[];' > crates/rari/snapshots/residual_lazy_sources.rs
    echo 'pub static RESIDUAL_LAZY_JS_SOURCES: &[(&str, &str)] = &[];' >> crates/rari/snapshots/residual_lazy_sources.rs

# Generate V8 snapshot for the current host (OS/arch-specific; do not reuse across platforms)
ci-generate-snapshot: ci-create-snapshot-placeholder
    cargo run --manifest-path tools/snapshot/Cargo.toml -- crates/rari/snapshots

# Rust fmt, clippy, and tests
ci-rust-check:
    cargo fmt --all -- --check
    cargo clippy --all-targets --all-features -- -D warnings
    cargo test --all-features

# Build the use-cache native addon and stage the platform package for CI
ci-prepare-use-cache-addon:
    #!/usr/bin/env bash
    set -euo pipefail

    os_name="$(uname -s)"
    case "$os_name" in
      Linux)
        platform=linux-x64
        napi_glob="*.linux-x64-gnu.node"
        package_dir="packages/use-cache-linux-x64"
        pkg_os=linux
        pkg_cpu=x64
        ;;
      MINGW*|MSYS*|CYGWIN*)
        platform=win32-x64
        napi_glob="*.win32-x64-msvc.node"
        package_dir="packages/use-cache-win32-x64"
        pkg_os=win32
        pkg_cpu=x64
        ;;
      *)
        echo "ci-prepare-use-cache-addon: unsupported OS: $os_name"
        exit 1
        ;;
    esac

    (cd crates/rari_use_cache && pnpm exec napi build --release --strip --platform)

    mkdir -p "$package_dir"
    shopt -s nullglob
    napi_files=(crates/rari_use_cache/$napi_glob)
    if [ ${#napi_files[@]} -eq 0 ]; then
      echo "expected napi artifact matching crates/rari_use_cache/$napi_glob"
      exit 1
    fi
    cp "${napi_files[0]}" "$package_dir/rari_use_cache.node"

    cat > "$package_dir/index.js" << 'EOF'
    import { createRequire } from 'node:module'
    import { dirname, join } from 'node:path'
    import { fileURLToPath } from 'node:url'

    const require = createRequire(import.meta.url)
    const __dirname = dirname(fileURLToPath(import.meta.url))

    const addon = require(join(__dirname, 'rari_use_cache.node'))

    export default addon
    EOF

    sed -e "s/{PLATFORM}/$platform/g" \
        -e 's/{VERSION}/0.0.0/g' \
        -e "s/{OS}/$pkg_os/g" \
        -e "s/{CPU}/$pkg_cpu/g" \
        .github/templates/package-json/use-cache-platform.json \
        > "$package_dir/package.json"

    rm -f crates/rari_use_cache/*.node crates/rari_use_cache/index.js crates/rari_use_cache/index.d.ts

# Build the Node packages required by CI
ci-build-packages:
    #!/usr/bin/env bash
    set -euo pipefail
    pnpm --filter=@rari/use-cache run build
    pnpm --filter=@rari/logger run build
    pnpm --filter=@rari/deploy run build
    pnpm --filter=rari run build
    pnpm --filter=create-rari-app run build

ci-typecheck-packages:
    # @rari/logger and @rari/deploy have no typecheck script (build-only via vp pack)
    pnpm --filter=rari run typecheck
    pnpm --filter=create-rari-app run typecheck

ci-lint-test: ci-build-packages
    pnpm lint
    pnpm test:unit:run

ci-verify-dist:
    #!/usr/bin/env bash
    set -euo pipefail
    test -d packages/use-cache/dist
    test -d packages/logger/dist
    test -d packages/deploy/dist
    test -d packages/rari/dist
    test -d packages/create-rari-app/dist
