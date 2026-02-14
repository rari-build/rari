<a href="https://rari.build" target="_blank">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset=".github/assets/rari-dark.svg">
    <source media="(prefers-color-scheme: light)" srcset=".github/assets/rari-light.svg">
    <img alt="rari" src=".github/assets/rari-light.svg" width="200">
  </picture>
</a>

> Runtime Accelerated Rendering Infrastructure

[![npm version](https://img.shields.io/npm/v/rari.svg)](https://www.npmjs.com/package/rari)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Discord](https://img.shields.io/badge/chat-discord-blue?style=flat&logo=discord)](https://discord.gg/GSh2Ak3b8Q)

**rari** is a React Server Components framework running on a Rust runtime. It has three layers: a Rust runtime (HTTP server, RSC renderer, and router with embedded V8), a React framework (app router, server actions, streaming/Suspense), and a build toolchain (Rolldown-Vite bundling, tsgo type checking). You write standard React — the runtime underneath is Rust instead of Node.

With proper app router support, true server-side rendering, and correct RSC semantics, rari delivers 46.5x higher throughput and 9.1x faster response times than Next.js.

## Features

- **App Router** - File-based routing with layouts, loading states, and error boundaries
- **True Server-Side Rendering** - Pre-rendered HTML with instant hydration
- **React Server Components** - Server components by default, client components when you need them
- **Rust-powered runtime** - HTTP server, RSC renderer, and routing written in Rust with embedded V8
- **Zero-config setup** - Works out of the box with pre-built binaries
- **Hot module reloading** - Instant feedback during development
- **node_modules support** - Standard npm package resolution, unlike most Rust-based JS runtimes
- **TypeScript-first** - Full type safety across server/client boundary
- **Cross-platform** - Supports macOS, Linux, and Windows
- **Streaming SSR** - Progressive rendering with Suspense boundaries
- **Loading States** - Automatic loading skeletons during navigation

## Quick Start

Create a new rari application in seconds:

```bash
npm create rari-app@latest my-app
cd my-app
npm run dev
```

That's it! Your app will be running at `http://localhost:5173`.

Visit [rari.build/docs](https://rari.build/docs) for complete documentation, guides, and examples.

## Documentation

**[Read the full documentation](https://rari.build/docs)** to learn more about:

- Getting started with rari
- App Router and file-based routing
- Server Components and Client Components
- Server Actions and data mutations
- Streaming SSR and Suspense
- Deployment and production optimization

## Why rari?

### Performance That Matters
- **9.1x faster response times** - 0.43ms vs 3.92ms average response
- **46.5x higher throughput** - 74,662 req/sec vs 1,605 req/sec under load
- **46.6x faster latency under load** - 0.67ms vs 31.17ms average latency
- **53% smaller bundles** - 264 KB vs 562 KB client JavaScript
- **3.7x faster builds** - 0.93s vs 3.47s production builds

### Developer Experience
- **App Router** - File-based routing with automatic code splitting
- **Zero configuration** - Create projects instantly with `create-rari-app`
- **Hot module reloading** - See changes instantly during development
- **Full TypeScript support** - Complete type safety across server/client boundary
- **Correct RSC semantics** - Server components by default, `'use client'` for interactivity
- **Cross-platform** - Works on macOS, Linux, and Windows

## Performance

rari delivers exceptional performance that significantly outperforms traditional React frameworks:

### Head-to-Head Comparison vs Next.js

> Benchmarks last updated: February 5, 2026

**Response Time (Single Request):**
| Metric | rari | Next.js | Improvement |
|--------|------|---------|-------------|
| **Average** | **0.43ms** | 3.92ms | **9.1x faster** |
| **P95** | 0.67ms | 4.55ms | **6.8x faster** |
| **Bundle Size** | 264 KB | 562 KB | **53% smaller** |

**Throughput Under Load (50 concurrent connections, 30s):**
| Metric | rari | Next.js | Improvement |
|--------|------|---------|-------------|
| **Requests/sec** | **74,662** | 1,605 | **46.5x higher** |
| **Avg Latency** | **0.67ms** | 31.17ms | **46.6x faster** |
| **P95 Latency** | **1.15ms** | 38.93ms | **33.9x faster** |
| **Errors** | 0 | 0 | Stable |

**Build Performance:**
| Metric | rari | Next.js | Improvement |
|--------|------|---------|-------------|
| **Build Time** | **0.93s** | 3.47s | **3.7x faster** |
| **Bundle Size** | 264 KB | 562 KB | **53% smaller** |

All benchmarks are reproducible. See [benchmarks/](https://github.com/rari-build/benchmarks) for methodology and scripts.

## Contributing

We welcome contributions! Here's how you can help:

- **Report Bugs** - Found an issue? [Open a bug report](https://github.com/rari-build/rari/issues/new)
- **Suggest Features** - Have ideas? [Share your suggestions](https://github.com/rari-build/rari/discussions)
- **Improve Docs** - Help make our documentation better
- **Submit PRs** - Check out our [Contributing Guide](.github/CONTRIBUTING.md)

## Community

- **Discord** - [Join our community](https://discord.gg/GSh2Ak3b8Q)
- **GitHub** - [Star the repo](https://github.com/rari-build/rari)
- **Documentation** - [rari.build/docs](https://rari.build/docs)

## Sponsors

rari is made possible by the support of these companies:

<div>
  <a href="https://get.neon.com/KDQudHN" target="_blank">
    <picture>
      <source media="(prefers-color-scheme: dark)" srcset="https://neon.com/brand/neon-logo-dark-color.svg">
      <source media="(prefers-color-scheme: light)" srcset="https://neon.com/brand/neon-logo-light-color.svg">
      <img width="250px" alt="Neon - Serverless Postgres" src="https://neon.com/brand/neon-logo-dark-color.svg">
    </picture>
  </a>
</div>

**[Neon](https://get.neon.com/KDQudHN)** - Serverless Postgres. Autoscaling, branching, and scale to zero.

---

Interested in sponsoring rari? [Get in touch](https://github.com/rari-build/rari/discussions) or support us on [GitHub Sponsors](https://github.com/sponsors/skiniks).

## License

MIT License - see [LICENSE](LICENSE) for details.
