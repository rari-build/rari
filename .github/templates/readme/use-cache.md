# @rari/use-cache

> Native `"use cache"` directive transform and runtime for [rari](https://github.com/rari-build/rari)

High-performance `"use cache"` support for React Server Components, powered by a Rust NAPI addon.

## Installation

Usually installed automatically with [rari](https://www.npmjs.com/package/rari). To install directly:

```bash
npm install @rari/use-cache
```

Enable caching in your rari Vite config:

```ts
import rari from 'rari/vite'
import { defineConfig } from 'vite'

export default defineConfig({
  plugins: [
    rari({
      experimental: {
        useCache: true,
      },
    }),
  ],
})
```

## What This Does

- Transforms `"use cache"` directives at build time via a native Rust addon
- Provides cache runtime helpers (`cacheLife`, `cacheTag`, revalidation APIs)
- Ships platform-specific optional dependencies for macOS, Linux, and Windows

You typically don't need to import this package in app code. rari loads the transform when `experimental.useCache` (or `experimental.useCacheRemote`) is enabled.

## Links

- **Documentation:** [rari.build/docs](https://rari.build/docs)
- **GitHub:** [github.com/rari-build/rari](https://github.com/rari-build/rari)
- **Discord:** [Join our community](https://discord.gg/GSh2Ak3b8Q)

## License

MIT License - see [LICENSE](https://github.com/rari-build/rari/blob/main/LICENSE) for details.
