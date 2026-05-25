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

**rari** is a React Server Components framework running on a Rust runtime. It has three layers: a Rust runtime (HTTP server, RSC renderer, and router with embedded V8), a React framework (app router, server actions, streaming/Suspense), and a build toolchain (Rolldown-powered Vite bundling, tsgo type checking). You write standard React, the runtime underneath is Rust instead of Node.

## Features

- **App Router** - File-based routing with layouts, loading states, and error boundaries
- **Server-Side Rendering** - Pre-rendered HTML with instant hydration
- **React Server Components** - Server components by default, client components when you need them
- **Rust-powered runtime** - HTTP server, RSC renderer, and routing written in Rust with embedded V8
- **Zero-config setup** - Works out of the box with pre-built binaries
- **Hot module reloading** - Instant feedback during development
- **node_modules support** - Standard npm package resolution without `npm:` specifier
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

## Performance

rari delivers exceptional performance that significantly outperforms traditional React frameworks:

### Head-to-Head Comparison vs Next.js

> Benchmarks last updated: May 25, 2026 (rari v0.13.11)

**Response Time (Single Request):**
| Metric | rari | Next.js | Improvement |
|--------|------|---------|-------------|
| **Average** | **0.12ms** | 2.17ms | **18.1x faster** |
| **P95** | 0.16ms | 2.37ms | **14.8x faster** |
| **Bundle Size** | 285 KB | 634 KB | **55% smaller** |

**Throughput Under Load (50 concurrent connections, 30s):**
| Metric | rari | Next.js | Improvement |
|--------|------|---------|-------------|
| **Requests/sec** | **97,826** | 1,452 | **67.4x higher** |
| **Avg Latency** | **0.51ms** | 34.46ms | **67.6x faster** |
| **P95 Latency** | **0.82ms** | 43.41ms | **52.9x faster** |
| **Errors** | 0 | 0 | Stable |

**Build Performance:**
| Metric | rari | Next.js | Improvement |
|--------|------|---------|-------------|
| **Build Time** | **1.75s** | 4.42s | **2.5x faster** |
| **Bundle Size** | 285 KB | 634 KB | **55% smaller** |

All benchmarks are reproducible. See [benchmarks/](https://github.com/rari-build/benchmarks) for methodology and tools.

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
    <img width="250px" alt="Neon - Serverless Postgres" src=".github/assets/neon.svg">
  </a>
</div>

**[Neon](https://get.neon.com/KDQudHN)** - Serverless Postgres. Autoscaling, branching, and scale to zero.

---

Interested in sponsoring rari? [Get in touch](https://github.com/rari-build/rari/discussions) or support us on [GitHub Sponsors](https://github.com/sponsors/skiniks).

## License

MIT License - see [LICENSE](LICENSE) for details.
