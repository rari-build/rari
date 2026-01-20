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
- **Documentation** (`docs/`) - Project documentation

## Development Setup

### Prerequisites

- **Node.js** 18+ (we recommend using the latest LTS version)
- **Rust** (latest stable version)
- **pnpm** (package manager - required for monorepo workspace management)

### Installation

1. **Clone the repository:**
   ```bash
   git clone https://github.com/rari-build/rari.git
   cd rari
   ```

2. **Install dependencies:**
   ```bash
   pnpm install
   ```

3. **Build the project:**
   ```bash
   pnpm run build
   ```

4. **Verify installation:**
   ```bash
   pnpm run typecheck
   pnpm run lint
   ```

## Project Structure

```
rari/
├── .github/           # GitHub workflows and templates
├── crates/            # Rust crates
│   └── rari/         # Main Rust runtime
├── docs/              # Documentation
├── examples/          # Example applications
│   └── basic-vite-rsc/
├── packages/          # TypeScript/JavaScript packages
│   ├── create-rari-app/  # CLI tool for creating new apps
│   └── rari/            # Main framework package
├── scripts/           # Build and release scripts
├── Cargo.toml         # Rust workspace configuration
├── pnpm-workspace.yaml # pnpm workspace configuration
└── package.json       # Root package.json
```

## Development Workflow

### Working with the Monorepo

This project uses **pnpm workspaces** for dependency management. All commands should be run from the root directory unless otherwise specified.

### Building the Project

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
cd examples/basic-vite-rsc

# Install dependencies (if not already done from root)
pnpm install

# Start development server
pnpm run dev
```

### Testing Changes

To test your changes:

1. **Build the project:**
   ```bash
   pnpm run build
   ```

2. **Run an example app:**
   ```bash
   cd examples/basic-vite-rsc
   pnpm run dev
   ```

3. **Test the CLI tool:**
   ```bash
   cd /tmp
   pnpm create rari-app test-app
   cd test-app
   pnpm run dev
   ```

### Rust Development

The Rust runtime is located in `crates/rari/`. Key commands:

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

TypeScript packages are in `packages/`. Key commands:

```bash
# Type check all packages
pnpm run typecheck

# Lint all packages
pnpm run lint

# Fix linting issues
pnpm run lint:fix
```

## Code Quality

### Linting and Formatting

We use multiple tools to maintain code quality:

- **ESLint** + **oxlint** for JavaScript/TypeScript linting & code formatting
- **Clippy** for Rust code

Run these commands before submitting:

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

```bash
# Run all tests
pnpm test

# Run Rust tests
cargo test

# Run TypeScript tests (if available)
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

Releases are managed by maintainers using our automated release script:

```bash
# Dry run (preview changes)
pnpm run release:dry

# Create actual release
pnpm run release
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
- [Architecture docs](../docs/) - Detailed technical documentation
- [Examples](../examples/) - Sample applications

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

### Working with Rust and TypeScript

- **Build order matters** - Rust crates need to be built before TypeScript packages
- **Use development mode** - The project automatically uses `cargo run` in development
- **Binary management** - Pre-built binaries are used in production
- **Cross-platform testing** - Test on different operating systems when possible

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
