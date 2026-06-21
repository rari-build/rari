# @rari/${PACKAGE_NAME}

> Platform-specific native addon for rari's 'use cache' transform on ${PLATFORM_NAME}

This package contains the native ${OS_NAME} ${ARCH} addon for [@rari/use-cache](https://github.com/rari-build/rari), providing high-performance 'use cache' directive transforms powered by Rust.

## Platform Support

- **Operating System:** ${OS_NAME}
- **Architecture:** ${ARCH}
- **Target:** ${DESCRIPTION}

## Installation

This package is automatically installed as an optional dependency when you install `@rari/use-cache`:

```bash
npm install @rari/use-cache
```

You typically don't need to install this package directly. The correct platform binary is selected automatically.

## What This Does

The native addon transforms React Server Components `"use cache"` directives at build time, providing:

- Fast directive detection via compiled Rust
- AST-based code transformation for cache wrappers
- Server reference registration

## Links

- **Documentation:** [rari.build/docs](https://rari.build/docs)
- **GitHub:** [github.com/rari-build/rari](https://github.com/rari-build/rari)
- **npm:** [npmjs.com/package/@rari/use-cache](https://www.npmjs.com/package/@rari/use-cache)

## License

MIT License - see [LICENSE](https://github.com/rari-build/rari/blob/main/LICENSE) for details.
