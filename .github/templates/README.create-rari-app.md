# create-rari-app

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
- Windows (x64 and ARM64)

If the binary download fails, check your internet connection and firewall settings.

## Contributing

We welcome contributions! See our [Contributing Guide](https://github.com/rari-build/rari/blob/main/.github/CONTRIBUTING.md) for details.

## License

MIT License - see [LICENSE](https://github.com/rari-build/rari/blob/main/LICENSE) for details.
