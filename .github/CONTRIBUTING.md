# Contributing to rari

Thank you for your interest in contributing to rari! We welcome contributions from the community and are excited to work with you.

rari is a React framework powered by a Rust runtime, designed for performance, scalability, and developer experience. This guide will help you get started with contributing to the project.

## Code of Conduct

Please read and follow our [Code of Conduct](./CODE_OF_CONDUCT.md) to ensure a welcoming and inclusive environment for all contributors.

## Project Overview

rari is a monorepo that consists of:

- **Rust crates** (`crates/`) - runtime engine and native addons (`rari`, `rari_error`, `rari_use_cache`)
- **TypeScript/JavaScript packages** (`packages/`) - framework, CLI, lint, deploy, use-cache, platform binaries
- **Tests** (`test/`) - Vitest unit/integration fixtures and Playwright e2e
- **Examples** (`examples/`) - sample applications
- **Web** (`web/`) - documentation site and landing page
- **Tools** (`tools/`) - build, snapshot, release, and binary packaging

## Naming conventions

Keep Rust and JS names aligned in meaning, but follow each ecosystem’s casing:

| Layer | Convention | Examples |
| --- | --- | --- |
| **Rust crates / modules / paths** | `snake_case` | `rari_use_cache`, `rari_error`, `tools/prepare_binaries` |
| **JS/TS packages / npm names / directories under `packages/`** | `kebab-case` | `@rari/use-cache`, `create-rari-app`, `use-cache-darwin-arm64` |
| **Paired crate ↔ package** | Same words, different separator | crate `rari_use_cache` ↔ package `@rari/use-cache` |

When adding a new feature that spans both sides, name the crate with underscores and the published package with dashes. Do not invent a third spelling.

## Development Setup

### Prerequisites

- **Node.js** `>=22.18.0` (see `engines` in `packages/rari/package.json`)
- **Rust nightly** via `rust-toolchain.toml` (rustup will install it automatically)
- **pnpm** via Corepack (`packageManager` is pinned in the root `package.json`)
- **just** (command runner, optional but recommended)

### Installation

1. **Clone the repository:**

   ```bash
   git clone https://github.com/rari-build/rari.git
   cd rari
   ```

2. **Run setup (recommended):**

   ```bash
   just setup
   ```

   This will:
   - Check prerequisites
   - Install Rust tools (cargo-nextest, cargo-machete, cargo-insta)
   - Enable Corepack and install pnpm dependencies

   Then build once:

   ```bash
   just build
   ```

   **Or manually:**

   ```bash
   corepack enable
   pnpm install
   just build   # or: pnpm run build after snapshot/binaries are present
   ```

3. **Verify installation:**
   ```bash
   just check
   # Or manually:
   pnpm run typecheck
   pnpm run lint
   ```

## Project Structure

```
rari/
├── .github/                 # Workflows, actions, PR/release templates
├── crates/                  # Rust crates
│   ├── rari/                # Main runtime
│   ├── rari_error/          # Shared error types
│   └── rari_use_cache/      # Native 'use cache' transform addon
├── packages/                # JS/TS packages
│   ├── rari/                # Main framework package
│   ├── create-rari-app/     # App scaffolding CLI
│   ├── use-cache/           # '@rari/use-cache' (pairs with rari_use_cache)
│   ├── use-cache-*/         # Platform-specific use-cache natives
│   ├── rari-*/              # Platform-specific rari binaries
│   ├── lint/, deploy/, logger/
├── test/                    # Unit, integration, e2e, fixtures
├── examples/                # Example applications
├── tools/                   # Build/release helpers
│   ├── prepare_binaries/    # Rust - package native binaries
│   ├── release/             # Rust - release automation
│   ├── snapshot/            # Rust - V8 snapshot generation
│   └── bundle-react-esm/    # JS - vendor React ESM for Flight
├── web/                     # Documentation website
├── justfile
├── Cargo.toml               # Rust workspace
├── pnpm-workspace.yaml      # pnpm workspace + dependency catalogs
└── package.json
```

`just build` runs React ESM bundling, V8 snapshot generation, then Rust and Node builds - order matters.

## Development Workflow

### Working with the Monorepo

This project uses **pnpm workspaces** for dependency management. All commands should be run from the root directory unless otherwise specified.

We use **just** as a command runner to simplify common tasks. Run `just` to see all available commands.

### Building the Project

**Using just (recommended):**

```bash
# Build everything (Rust + Node.js)
just build

# Build only Rust crates
just build-rust

# Build only Node.js packages
just build-node

# Build specific packages
just build-rari
just build-create-rari-app
just build-web
```

**Or manually:**

```bash
# Build all packages and crates (prefer just build - includes snapshot + React ESM)
pnpm run build

# Build only TypeScript packages
pnpm -r run build

# Build only Rust crates (requires a current V8 snapshot under crates/rari/snapshots/)
cargo build --release
```

### Running Examples

```bash
# Navigate to an example
cd examples/app-router-example

# Install dependencies (if not already done from root)
pnpm install

# Start development server
pnpm run dev
```

### Running the Web Documentation Site

```bash
# Start the web dev server
just dev

# Or manually:
pnpm --filter @rari/web dev
```

### Testing Changes

To test your changes:

1. **Build the project:**

   ```bash
   just build
   # Or: pnpm run build
   ```

2. **Run an example app:**

   ```bash
   cd examples/app-router-example
   pnpm run dev
   ```

3. **Test the CLI tool:**

   ```bash
   cd /tmp
   pnpm create rari-app test-app
   cd test-app
   pnpm run dev
   ```

4. **Run the rari CLI directly:**
   ```bash
   just run --help
   # Or in release mode:
   just run-release --help
   ```

### Rust Development

The Rust runtime is located in `crates/rari/`. Key commands:

**Using just (recommended):**

```bash
# Lint Rust code
just lint-rust

# Fix Rust formatting and linting
just fix-rust

# Run Rust tests with nextest
just test-rust

# Run all Rust tests (including doc tests)
just test-rust-all

# Check for unused dependencies
just machete

# Build release version
just build-rust-release
```

**Or manually:**

```bash
# Format Rust code
cargo fmt

# Check for issues
cargo clippy

# Run Rust tests
cargo test

# Build release version
cargo build --release
```

### TypeScript Development

TypeScript packages are in `packages/` and `web/`. Key commands:

**Using just (recommended):**

```bash
# Type check all packages
just typecheck

# Lint all packages
just lint-node

# Fix linting issues
just fix-node

# Check for unused dependencies and exports
just knip
```

**Or manually:**

```bash
# Type check all packages
pnpm run typecheck

# Lint all packages
pnpm run lint

# Fix linting issues
pnpm run lint:fix

# Check for unused dependencies
pnpm knip
```

## Code Quality

### Linting and Formatting

We use multiple tools to maintain code quality:

- **vite-plus (`vp fmt` / `vp lint`)** + **ESLint** + **oxlint** for JS/TS and shared repo lint
- **Clippy** + **rustfmt** for Rust
- **cargo-machete** for unused Rust dependencies
- **knip** for unused TypeScript dependencies and exports
- Shared JS dependency versions live in **pnpm catalogs** (`pnpm-workspace.yaml`) - prefer `catalog:` / `catalog:<name>` over hardcoding versions in package manifests

Run these commands before submitting:

**Using just (recommended):**

```bash
# Lint all code (Rust + Node.js)
just lint

# Fix auto-fixable issues
just fix

# Type check
just typecheck

# Run all checks (lint + test + typecheck)
just check

# Quick development check (faster)
just quick-check
```

**Or manually:**

```bash
# Lint all code
pnpm run lint

# Fix auto-fixable issues
pnpm run lint:fix

# Type check
pnpm run typecheck
```

### Rust-specific Guidelines

- Follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Crate and module names use `snake_case`
- Use `cargo fmt`; run `cargo clippy` and fix warnings
- Add tests for new functionality
- Document public APIs with doc comments
- Deno-related crates are pinned as a set in the workspace `Cargo.toml` - bump them together, not via ad-hoc `cargo update` on a single Deno crate

### TypeScript Guidelines

- Package names and `packages/*` directories use `kebab-case`
- Use TypeScript strict mode; prefer type safety over `any`
- Follow the existing code style (`vp fmt` / ESLint)
- Add JSDoc comments for public APIs
- Write tests for new features under `test/`
- When a package wraps a Rust crate, keep the npm name the dashed form of the crate name (`rari_use_cache` → `@rari/use-cache`)

## Testing

### Running Tests

**Using just (recommended):**

```bash
# Run all tests (Rust + Node.js)
just test

# Run only Rust tests with nextest
just test-rust

# Run all Rust tests (including doc tests)
just test-rust-all

# Run only Node.js tests
just test-node

# Run specific test by filter
just test-filter "test_name"

# Watch tests (re-run on file changes)
just test-watch

# Run tests with coverage
just test-coverage
```

**Or manually:**

```bash
# Run all tests
pnpm test

# Run Rust tests
cargo test

# Run TypeScript tests
pnpm -r run test
```

### Writing Tests

- **Rust:** unit tests inline with `#[cfg(test)]`, or under crate `tests/` when integration-style
- **TypeScript unit/integration:** Vitest via `vp test` - live under `test/unit/`, `test/integration/`, with helpers in `test/helpers/` and fixtures in `test/fixtures/`
- **E2E:** Playwright under `test/e2e/` (`pnpm test:e2e` / `just` recipes that call it)
- Prefer colocating coverage with the subsystem you change (e.g. Vite transform bugs → `test/unit/vite/`)
- Cover edge cases and failure modes, not only the happy path

## Commit Guidelines

We follow the [Conventional Commits](https://www.conventionalcommits.org/) specification:

### Commit Message Format

```
<type>(<scope>): <description>

[optional body]

[optional footer(s)]
```

### Types

- **feat**: New features
- **fix**: Bug fixes
- **docs**: Documentation changes
- **style**: Code style changes (formatting, etc.)
- **refactor**: Code refactoring
- **test**: Test additions or changes
- **chore**: Build process or auxiliary tool changes
- **perf**: Performance improvements

### Scopes

Prefer a concrete subsystem over a vague umbrella:

- **rari** / **rsc** / **cache** - runtime, Flight/Fizz, `'use cache'`
- **vite** - Vite plugin / RSC transforms / HMR
- **cli** - `rari` CLI or `create-rari-app`
- **ci** / **release** / **release-notes** / **dependencies**
- **docs** / **examples** / **web**

Use the package or crate name when it helps (`use-cache`, `rari_error`), still matching that ecosystem’s casing in prose if you mention paths.

### Examples

```bash
feat(vite): register inline use-server actions for export default
fix(rsc): fail loudly on flight client patch drift
docs: document crate vs package naming
chore(dependencies): bump react vendor pins
```

## Release Process

Releases are managed by maintainers using our automated release tools:

**Using just (recommended):**

```bash
# Dry run (preview changes)
just release-dry

# Create actual release
just release

# Prepare binaries for release
just prepare-binaries

# Generate changelog
just changelog

# Preview unreleased changes
just changelog-preview
```

**Or manually:**

```bash
# Run release tool
cargo run --release --manifest-path tools/release/Cargo.toml --bin release

# Dry run
cargo run --release --manifest-path tools/release/Cargo.toml --bin release -- --dry-run
```

The release process:

1. Bumps version numbers
2. Generates changelog
3. Creates git tags
4. Publishes to npm
5. Uploads binaries to GitHub releases

### Manual release notes (hybrid)

For minors/majors, add curated notes under `.github/release-notes/` before releasing:

```bash
cp .github/release-notes/_template.md .github/release-notes/0.15.0.md
# edit Highlights / Breaking Changes, then:
just release
```

Lookup order: `--notes-file` / `RELEASE_NOTES_FILE` → `<tag>.md` (scoped `/` → `-`) → `<version>.md`.

Manual notes are prepended to the GitHub release body (above the auto-generated commit list) and injected into `CHANGELOG.md` under the new version heading. Patch releases can skip this and stay cliff-only.

The release tool **prepends** only the new version section into each package `CHANGELOG.md`, so older curated Highlights / Breaking Changes are kept. It also passes a package `--tag-pattern` (e.g. `^rari@`) so binary `v*` tags and other packages are not mixed into the same changelog.

`just changelog` / `just changelog-preview` only print unreleased commits to stdout - they do not rewrite `CHANGELOG.md`.

## Pull Request Guidelines

### Before Submitting

1. **Test your changes** thoroughly
2. **Run all quality checks:**
   ```bash
   just check
   # Or manually:
   pnpm run lint
   pnpm run typecheck
   pnpm run build
   ```
3. **Write descriptive commit messages**
4. **Update documentation** if needed
5. **Add tests** for new features

### PR Requirements

- **Small, focused changes** are preferred
- **Clear description** of what the PR does
- **Link to relevant issues** (if applicable)
- **Include screenshots** for UI changes
- **Update documentation** for API changes
- **Add changelog entry** for user-facing changes

### Review Process

1. **Automated checks** must pass
2. **Code review** by maintainers
3. **Testing** in different environments
4. **Final approval** before merge

## Getting Help

### Documentation

- [README](../README.md) - Project overview and quick start
- [Website](https://rari.build/docs) - Full documentation and guides
- [Examples](../examples/) - Sample applications
- [justfile](../justfile) - All available commands

### Communication

- **GitHub Issues** - Bug reports and feature requests
- **GitHub Discussions** - General questions and community discussion
- **Discord** - Real-time chat with the community

### Reporting Issues

When reporting bugs:

1. **Check existing issues** first
2. **Use the issue template**
3. **Include reproduction steps**
4. **Provide system information**
5. **Include relevant logs**

### Feature Requests

For new features:

1. **Search existing issues** and discussions
2. **Describe the use case** clearly
3. **Explain the benefits** to users
4. **Consider implementation complexity**
5. **Be open to feedback** and alternatives

## Development Tips

### Using just

The `justfile` provides convenient commands for all common tasks. Run `just` or `just --list` to see all available commands.

Key commands:

- `just setup` - One-time setup for new contributors
- `just build` - Build everything
- `just test` - Run all tests
- `just lint` - Lint all code
- `just fix` - Auto-fix linting issues
- `just check` - Run all checks (CI workflow)
- `just dev` - Start web dev server

### Working with Rust and TypeScript

- **Build order matters** - `just build` runs React ESM bundling + V8 snapshot before Rust/Node; prefer it over ad-hoc partial builds when unsure
- **Naming** - underscores in Rust, dashes in JS (see [Naming conventions](#naming-conventions))
- **Development mode** - local tooling often uses `cargo run`; published installs use platform binary packages
- **Cross-platform** - CI covers Linux and Windows Namespace runners; test Windows-sensitive paths when you touch them
- **Use just** - recipes encode dependencies and build order

### Performance Considerations

- **Profile before optimizing** - Use proper profiling tools
- **Benchmark changes** - Measure performance impact
- **Memory efficiency** - Consider memory usage in Rust code
- **Bundle size** - Keep TypeScript bundles optimized

### Debugging

- **Rust debugging** - Use `RUST_LOG=debug` for detailed logs
- **TypeScript debugging** - Use browser dev tools and VS Code debugger
- **Integration testing** - Test the full pipeline from CLI to runtime

## Recognition

Contributors are recognized in:

- **README.md** - Major contributors
- **CHANGELOG.md** - Credit for specific changes
- **GitHub releases** - Acknowledgment in release notes

Thank you for contributing to rari! Your efforts help make rari faster and more accessible for everyone.
