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
[![CodSpeed](https://img.shields.io/badge/CodSpeed-performance-blue)](https://codspeed.io/rari-build/rari?utm_source=badge)

**rari** is a high-performance React Server Components framework powered by a Rust runtime. Built for performance, scalability, and developer experience.

With proper app router support, true server-side rendering, and correct RSC semantics, rari delivers 8.7x higher throughput and 7.5x faster response times than Next.js.

## Features

- **App Router** - File-based routing with layouts, loading states, and error boundaries
- **True Server-Side Rendering** - Pre-rendered HTML with instant hydration
- **React Server Components** - Server components by default, client components when you need them
- **Rust-powered runtime** - Persistent runtime for maximum performance
- **Zero-config setup** - Works out of the box with pre-built binaries
- **Hot module reloading** - Instant feedback during development
- **Universal NPM package support** - Use any npm package seamlessly
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
- **7.5x faster response times** - 0.35ms vs 2.64ms average response
- **8.7x higher throughput** - 14,085 req/sec vs 1,624 req/sec under load
- **8.7x faster latency under load** - 3.55ms vs 30.79ms average latency
- **53% smaller bundles** - 266 KB vs 565 KB client JavaScript
- **2.9x faster builds** - 1.41s vs 4.10s production builds

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

> Benchmarks last updated: January 26, 2026

**Response Time (Single Request):**
| Metric | rari | Next.js | Improvement |
|--------|------|---------|-------------|
| **Average** | **0.35ms** | 2.64ms | **7.5x faster** |
| **P95** | 0.38ms | 3.44ms | **9.1x faster** |
| **Bundle Size** | 266 KB | 565 KB | **53% smaller** |

**Throughput Under Load (50 concurrent connections, 30s):**
| Metric | rari | Next.js | Improvement |
|--------|------|---------|-------------|
| **Requests/sec** | **14,085** | 1,624 | **8.7x higher** |
| **Avg Latency** | **3.55ms** | 30.79ms | **8.7x faster** |
| **P95 Latency** | **5.78ms** | 38.43ms | **6.6x faster** |
| **Errors** | 0 | 0 | Stable |

**Build Performance:**
| Metric | rari | Next.js | Improvement |
|--------|------|---------|-------------|
| **Build Time** | **1.41s** | 4.10s | **2.9x faster** |
| **Bundle Size** | 266 KB | 565 KB | **53% smaller** |

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

## License

MIT License - see [LICENSE](LICENSE) for details.
