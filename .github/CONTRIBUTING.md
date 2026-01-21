# Contributing to rari

Thank you for your interest in contributing to rari! We welcome contributions from the community and are excited to work with you.

rari is a React framework powered by a Rust runtime, designed for performance, scalability, and developer experience. This guide will help you get started with contributing to the project.

## Code of Conduct

Please read and follow our [Code of Conduct](./CODE_OF_CONDUCT.md) to ensure a welcoming and inclusive environment for all contributors.

## Project Overview

rari is a monorepo that consists of:

- **Rust Core** (`crates/`) - The high-performance runtime engine
- **TypeScript/JavaScript Packages** (`packages/`) - Framework tooling and APIs
- **Examples** (`examples/`) - Sample applications and demonstrations
- **Web** (`web/`) - Documentation website and landing page
- **Tools** (`tools/`) - Build and release automation tools

## Development Setup

### Prerequisites

- **Node.js** 18+ (we recommend using the latest LTS version)
- **Rust** (latest stable version)
- **pnpm** (package manager - required for monorepo workspace management)
- **just** (command runner - optional but recommended for easier development)

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
   - Enable corepack and install pnpm dependencies
   - Build all packages

   **Or manually:**
   ```bash
   # Install dependencies
   pnpm install

   # Build the project
   pnpm run build
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
├── .github/           # GitHub workflows and templates
├── crates/            # Rust crates
│   └── rari/         # Main Rust runtime
├── examples/          # Example applications
│   └── app-router-example/
├── packages/          # TypeScript/JavaScript packages
│   ├── create-rari-app/  # CLI tool for creating new apps
│   ├── rari/            # Main framework package
│   └── rari-{platform}/ # Platform-specific binary packages
├── tools/             # Build and release automation
│   ├── prepare-binaries/ # Binary preparation tool
│   └── release/         # Release automation tool
├── web/               # Documentation website
├── justfile           # Command runner recipes
├── Cargo.toml         # Rust workspace configuration
├── pnpm-workspace.yaml # pnpm workspace configuration
└── package.json       # Root package.json
```

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
# Build all packages and crates
pnpm run build

# Build only TypeScript packages
pnpm -r run build

# Build only Rust crates
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

- **ESLint** + **oxlint** for JavaScript/TypeScript linting & code formatting
- **Clippy** for Rust linting
- **rustfmt** for Rust code formatting
- **cargo-machete** for detecting unused Rust dependencies
- **knip** for detecting unused TypeScript dependencies and exports

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
- Use `cargo fmt` to format code
- Run `cargo clippy` and fix all warnings
- Add tests for new functionality
- Document public APIs with doc comments

### TypeScript Guidelines

- Use TypeScript strict mode
- Prefer type safety over any
- Follow the existing code style
- Add JSDoc comments for public APIs
- Write tests for new features

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

- **Rust tests** should be in `tests/` directories or inline with `#[cfg(test)]`
- **TypeScript tests** should use the same testing framework as existing code
- Add integration tests for new features
- Test edge cases and error conditions

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

- **core**: Rust runtime changes
- **cli**: CLI tool changes
- **vite**: Vite plugin changes
- **docs**: Documentation changes
- **examples**: Example app changes

### Examples

```bash
feat(core): add streaming support for RSC
fix(cli): resolve path resolution issue on Windows
docs: update installation instructions
chore(deps): update dependencies
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

- **Build order matters** - Rust crates need to be built before TypeScript packages
- **Use development mode** - The project automatically uses `cargo run` in development
- **Binary management** - Pre-built binaries are used in production
- **Cross-platform testing** - Test on different operating systems when possible
- **Use just commands** - They handle dependencies and build order automatically

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
