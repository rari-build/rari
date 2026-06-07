# ${PACKAGE_NAME}

> Platform-specific binary for rari on ${PLATFORM_NAME}

This package contains the native ${OS_NAME} ${ARCH} binary for [rari](https://github.com/rari-build/rari), a high-performance React Server Components framework powered by Rust.

## Platform Support

- **Operating System:** ${OS_NAME}
- **Architecture:** ${ARCH}
- **Target:** ${DESCRIPTION}

## Installation

This package is automatically installed as an optional dependency when you install the main `rari` package:

```bash
npm install rari
```

You typically don't need to install this package directly. The main `rari` package will automatically download the correct binary for your platform.

## Usage

This package is used internally by the `rari` CLI. After installing `rari`, you can use it directly:

```bash
# Development server
npx rari dev

# Production build
npx rari build

# Start production server
npx rari start
```

### Quick Start

Create a new rari application:

```bash
npm create rari-app@latest my-app
cd my-app
npm run dev
```

Visit [rari.build](https://rari.build) for complete documentation.

## Links

- **Documentation:** [rari.build/docs](https://rari.build/docs)
- **GitHub:** [github.com/rari-build/rari](https://github.com/rari-build/rari)
- **Discord:** [Join our community](https://discord.gg/GSh2Ak3b8Q)
- **npm:** [npmjs.com/package/rari](https://www.npmjs.com/package/rari)

## License

MIT License - see [LICENSE](https://github.com/rari-build/rari/blob/main/LICENSE) for details.
