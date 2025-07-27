# Rari

> Runtime Accelerated Rendering Infrastructure

[![npm version](https://img.shields.io/npm/v/rari.svg)](https://www.npmjs.com/package/rari)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Discord](https://img.shields.io/badge/chat-discord-blue?style=flat&logo=discord)](https://discord.gg/GSh2Ak3b8Q)

**Rari** is a high-performance React Server Components framework powered by a Rust runtime. Built for performance, scalability, and developer experience.

## Features

- **Performance-optimized React Server Components** with Rust runtime
- **Zero-config setup** - works out of the box with pre-built binaries
- **Hot module reloading** for instant development feedback
- **Universal NPM package support** - use any npm package seamlessly
- **TypeScript-first** with full type safety
- **Streaming support** for better user experience
- **Memory efficient** with automatic optimizations
- **Cross-platform** - supports macOS, Linux, and Windows

## Quick Start

Create a new Rari application in seconds:

```bash
npm create rari-app@latest my-app
cd my-app
npm run dev
```

That's it! Your app will be running at `http://localhost:5173`.

## Installation

### Option 1: Use create-rari-app (Recommended)
```bash
npm create rari-app@latest my-app
# or
pnpm create rari-app my-app
# or
yarn create rari-app my-app
```

### Option 2: Add to existing project
```bash
npm install rari
# or
pnpm add rari
```

The Rari binary is automatically downloaded and configured during installation. No additional setup required!

## Documentation

- [Getting Started](docs/public/content/getting-started.md)
- [Contributing](.github/CONTRIBUTING.md)

## Why Rari?

### Performance That Matters
- **Rust-powered runtime** - 4.04x faster response times under load vs Next.js
- **Memory efficient** - Superior garbage collection and resource management
- **Streaming RSC** - Components render as they resolve, no waiting

### Developer Experience
- **Zero configuration** - Create projects instantly with `create-rari-app`
- **Hot module reloading** - See changes instantly during development
- **Full TypeScript support** - Complete type safety out of the box
- **Universal package support** - Use any npm package seamlessly
- **Cross-platform** - Works on macOS, Linux, and Windows

### React Server Components Done Right
- **True server-side rendering** with async/await support
- **Automatic serialization** between server and client
- **Streaming support** with React Suspense
- **Error boundaries** for graceful error handling

## Performance

Rari delivers exceptional performance that significantly outperforms traditional React frameworks:

### Head-to-Head Comparison vs Next.js

**Server Response Performance:**
- **Single Request**: Rari 2.15ms vs Next.js 4.88ms (**2.27x faster**)
- **Concurrent Load Latency**: Rari 4.23ms vs Next.js 17.11ms (**4.04x faster**)
- **Throughput**: Rari 10,586 req/s vs Next.js 2,832 req/s (**3.74x more requests**)

**Development & Build Performance:**
- **Build Time**: Rari 1.59s vs Next.js 9.22s (**5.80x faster**)
- **Bundle Size**: Rari 400KB vs Next.js 742KB (**46% smaller**)

**Production Metrics:**
```
Single Request Response:   2.15ms           (individual requests)
Concurrent Load Latency:   4.23ms           (50 connections)
Sustained Throughput:      10,586 req/sec   (30s load test)
Build Performance:         1.59s            (production build)
Bundle Optimization:       400KB            (optimized output)
Zero Errors:              100% success      (all test scenarios)
```

**Detailed Performance Metrics:**

**Response Time Analysis:**
- **Individual requests**: 2.15ms average (range: 1.34-3.09ms)
- **Under load (50 concurrent)**: 4.23ms average latency
- **Throughput capacity**: 10,586 requests/second sustained
- **Response consistency**: 99th percentile under 46ms

**Concurrent Load Testing:**
- **Sustained throughput**: 10,586 requests/second with 100% success rate
- **Load test duration**: 30 seconds with 50 concurrent connections
- **Latency under load**: 4.23ms average, 4.04x faster than Next.js
- **Perfect reliability**: Zero errors or timeouts across all tests

**Development & Production:**
- **Build performance**: 5.80x faster builds (1.59s vs 9.22s)
- **Bundle efficiency**: 46% smaller output with same functionality
- **Hot reload**: Instant feedback during development
- **Bundle optimization**: Automatic tree shaking and code splitting

## Project Structure

```
my-rari-app/
├── src/
│   ├── components/
│   │   ├── ServerTime.tsx    # Server components ('use server')
│   │   └── Welcome.tsx       # Client components
│   ├── styles/
│   │   └── index.css
│   ├── App.tsx               # Root component
│   └── main.tsx              # Client entry
├── vite.config.ts            # Vite + Rari configuration
└── package.json
```

## Examples

### Server Component
```tsx
'use server'

export default async function UserProfile({ userId }: { userId: string }) {
  // This runs on the server
  const user = await fetch(`/api/users/${userId}`).then(r => r.json())

  return (
    <div>
      <h1>{user.name}</h1>
      <p>{user.email}</p>
    </div>
  )
}
```

### Client Component
```tsx
import { useState } from 'react'

export default function Counter() {
  const [count, setCount] = useState(0)

  return (
    <button onClick={() => setCount(count + 1)}>
      Count:
      {' '}
      {count}
    </button>
  )
}
```

### NPM Package Usage
```tsx
'use server'

import MarkdownIt from 'markdown-it'

export default async function BlogPost({ content }: { content: string }) {
  const md = MarkdownIt()
  const html = md.render(content)

  return <div dangerouslySetInnerHTML={{ __html: html }} />
}
```

## Configuration

Rari works with zero configuration, but you can customize it:

```typescript
// vite.config.ts
import { rari } from 'rari'
import { defineConfig } from 'vite'

export default defineConfig({
  plugins: [
    rari({
      // Optional configuration
    })
  ]
})
```

## Commands

### Development
```bash
npm run dev
```

### Production
```bash
npm run build
npm start
```

### Environment Variables
- `PORT` - Server port (default: 3000)
- `NODE_ENV` - Environment mode (development/production)
- `RUST_LOG` - Rust logging level (default: info)

## Development

### Prerequisites
- Node.js 20+
- Rust (for core development)
- pnpm (recommended)

### Building from Source
```bash
git clone https://github.com/rari-build/rari.git
cd rari
pnpm install
pnpm build
```

### Running Examples
```bash
cd examples/basic-vite-rsc
pnpm install
pnpm dev
```

## Contributing

We welcome contributions! Please see our [Contributing Guide](.github/CONTRIBUTING.md) for details.

## License

MIT License - see [LICENSE](LICENSE) for details.
