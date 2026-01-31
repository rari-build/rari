use anyhow::Result;
use std::path::Path;
use tokio::fs;

pub async fn generate_rari_readme(package_path: &Path) -> Result<()> {
    let readme_content = r#"# rari

> Runtime Accelerated Rendering Infrastructure

**rari** is a high-performance React Server Components framework powered by a Rust runtime. Built for performance, scalability, and developer experience.

## Quick Start

Create a new rari application in seconds:

```bash
npm create rari-app@latest my-app
cd my-app
npm run dev
```

That's it! Your app will be running at `http://localhost:5173`.

Visit [rari.build/docs](https://rari.build/docs) for complete documentation, guides, and examples.

## Installation

Install rari in an existing project:

```bash
npm install rari react react-dom
```

Or use the CLI to create a new project:

```bash
npm create rari-app@latest
```

## Documentation

**[Read the full documentation](https://rari.build/docs)** to learn more about:

- Getting started with rari
- App Router and file-based routing
- Server Components and Client Components
- Server Actions and data mutations
- Streaming SSR and Suspense
- Deployment and production optimization

## Contributing

We welcome contributions! Here's how you can help:

- **Report Bugs** - Found an issue? [Open a bug report](https://github.com/rari-build/rari/issues/new)
- **Suggest Features** - Have ideas? [Share your suggestions](https://github.com/rari-build/rari/discussions)
- **Improve Docs** - Help make our documentation better
- **Submit PRs** - Check out our [Contributing Guide](https://github.com/rari-build/rari/blob/main/.github/CONTRIBUTING.md)

## Community

- **Discord** - [Join our community](https://discord.gg/GSh2Ak3b8Q)
- **GitHub** - [Star the repo](https://github.com/rari-build/rari)
- **Documentation** - [rari.build/docs](https://rari.build/docs)

## License

MIT License - see [LICENSE](https://github.com/rari-build/rari/blob/main/LICENSE) for details.
"#;

    let readme_path = package_path.join("README.md");
    fs::write(readme_path, readme_content).await?;
    Ok(())
}

pub async fn generate_create_rari_app_readme(package_path: &Path) -> Result<()> {
    let readme_content = r#"# create-rari-app

> Scaffold a new rari application with zero configuration

The fastest way to get started with [rari](https://github.com/rari-build/rari), a high-performance React Server Components framework powered by Rust.

## Usage

Create a new rari application interactively:

```bash
npm create rari-app@latest
```

Or specify a project name directly:

```bash
npm create rari-app@latest my-app
```

With other package managers:

```bash
# pnpm
pnpm create rari-app

# yarn
yarn create rari-app

# bun
bun create rari-app
```

## What You Get

Running `create-rari-app` sets up a complete rari project with:

- **App Router structure** - Pre-configured file-based routing
- **TypeScript support** - Full type safety out of the box
- **Development server** - Hot module reloading for instant feedback
- **Example components** - Server and client component examples
- **Production ready** - Optimized build configuration included

## Project Structure

```
my-app/
├── app/
│   ├── layout.tsx          # Root layout
│   ├── page.tsx            # Home page
│   └── global.css          # Global styles
├── public/                 # Static assets
├── package.json
├── tsconfig.json
└── vite.config.ts
```

## Getting Started

After creating your project:

```bash
cd my-app
npm install
npm run dev
```

Your app will be running at `http://localhost:5173`.

## Available Commands

- `npm run dev` - Start development server with hot reloading
- `npm run build` - Build for production
- `npm run start` - Start production server
- `npm run typecheck` - Run TypeScript type checking

## Templates

`create-rari-app` includes a default template with:

- React 19 with Server Components
- TypeScript configuration
- Vite integration
- Example pages and components
- CSS support

## Documentation

Visit [rari.build/docs](https://rari.build/docs) for:

- Complete documentation
- Guides and tutorials
- API reference
- Deployment guides
- Performance tips

## Community

- **Discord** - [Join our community](https://discord.gg/GSh2Ak3b8Q)
- **GitHub** - [rari-build/rari](https://github.com/rari-build/rari)
- **Documentation** - [rari.build/docs](https://rari.build/docs)

## Troubleshooting

### Installation Issues

If you encounter issues during installation:

1. Ensure you're using Node.js 22 or higher: `node --version`
2. Clear npm cache: `npm cache clean --force`
3. Try with a different package manager (pnpm, yarn, or bun)

### Platform-Specific Binaries

rari automatically downloads the correct binary for your platform. Supported platforms:

- macOS (Intel and Apple Silicon)
- Linux (x64 and ARM64)
- Windows (x64)

If the binary download fails, check your internet connection and firewall settings.

## Contributing

We welcome contributions! See our [Contributing Guide](https://github.com/rari-build/rari/blob/main/.github/CONTRIBUTING.md) for details.

## License

MIT License - see [LICENSE](https://github.com/rari-build/rari/blob/main/LICENSE) for details.
"#;

    let readme_path = package_path.join("README.md");
    fs::write(readme_path, readme_content).await?;
    Ok(())
}

pub async fn copy_license(package_path: &Path) -> Result<()> {
    let root_license = Path::new("LICENSE");
    let package_license = package_path.join("LICENSE");
    fs::copy(root_license, package_license).await?;
    Ok(())
}

pub async fn generate_package_files(package_name: &str, package_path: &Path) -> Result<()> {
    match package_name {
        "rari" => {
            generate_rari_readme(package_path).await?;
            copy_license(package_path).await?;
        }
        "create-rari-app" => {
            generate_create_rari_app_readme(package_path).await?;
            copy_license(package_path).await?;
        }
        _ => {
            anyhow::bail!("Unknown package: {}", package_name);
        }
    }
    Ok(())
}

pub async fn cleanup_package_files(package_path: &Path) -> Result<()> {
    let readme_path = package_path.join("README.md");
    let license_path = package_path.join("LICENSE");

    let _ = fs::remove_file(readme_path).await;
    let _ = fs::remove_file(license_path).await;

    Ok(())
}
