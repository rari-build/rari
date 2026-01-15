# Rari

> Runtime Accelerated Rendering Infrastructure

[![npm version](https://img.shields.io/npm/v/rari.svg)](https://www.npmjs.com/package/rari)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Discord](https://img.shields.io/badge/chat-discord-blue?style=flat&logo=discord)](https://discord.gg/GSh2Ak3b8Q)

**Rari** is a high-performance React Server Components framework powered by a Rust runtime. Built for performance, scalability, and developer experience.

With proper app router support, true server-side rendering, and correct RSC semantics, Rari delivers 12x higher throughput and 2x faster response times than Next.js.

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

Create a new Rari application in seconds:

```bash
npm create rari-app@latest my-app
cd my-app
npm run dev
```

That's it! Your app will be running at `http://localhost:5173`.

Visit [rari.build/docs](https://rari.build/docs) for complete documentation, guides, and examples.

## Documentation

**[Read the full documentation](https://rari.build/docs)** to learn more about:

- Getting started with Rari
- App Router and file-based routing
- Server Components and Client Components
- Server Actions and data mutations
- Streaming SSR and Suspense
- Deployment and production optimization

## Why Rari?

### Performance That Matters
- **2x faster response times** - 1.32ms vs 2.63ms average response
- **12x higher throughput** - 21,263 req/sec vs 1,760 req/sec under load
- **14x faster latency under load** - 1.94ms vs 27.89ms average latency
- **66% smaller bundles** - 28.1 KB vs 82.0 KB client JavaScript
- **2.6x faster builds** - 1.73s vs 4.43s production builds

### Developer Experience
- **App Router** - File-based routing with automatic code splitting
- **Zero configuration** - Create projects instantly with `create-rari-app`
- **Hot module reloading** - See changes instantly during development
- **Full TypeScript support** - Complete type safety across server/client boundary
- **Correct RSC semantics** - Server components by default, `'use client'` for interactivity
- **Cross-platform** - Works on macOS, Linux, and Windows

## Performance

Rari delivers exceptional performance that significantly outperforms traditional React frameworks:

### Head-to-Head Comparison vs Next.js

> Benchmarks last updated: January 14, 2026

**Response Time (Single Request):**
| Metric | Rari | Next.js | Improvement |
|--------|------|---------|-------------|
| **Average** | **1.32ms** | 2.63ms | **2.0x faster** |
| **P95** | 3.36ms | 4.06ms | **1.2x faster** |
| **Bundle Size** | 28.1 KB | 82.0 KB | **66% smaller** |

**Throughput Under Load (50 concurrent connections, 30s):**
| Metric | Rari | Next.js | Improvement |
|--------|------|---------|-------------|
| **Requests/sec** | **21,263** | 1,760 | **12.1x higher** |
| **Avg Latency** | **1.94ms** | 27.89ms | **14.4x faster** |
| **P99 Latency** | **<2ms** | 28ms | **14x faster** |
| **Errors** | 0 | 0 | Stable |

**Build Performance:**
| Metric | Rari | Next.js | Improvement |
|--------|------|---------|-------------|
| **Build Time** | **1.73s** | 4.43s | **2.6x faster** |
| **Bundle Size** | 433 KB | 543 KB | **20% smaller** |

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
