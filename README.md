# Rari

> Runtime Accelerated Rendering Infrastructure

[![npm version](https://img.shields.io/npm/v/rari.svg)](https://www.npmjs.com/package/rari)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Discord](https://img.shields.io/badge/chat-discord-blue?style=flat&logo=discord)](https://discord.gg/GSh2Ak3b8Q)

**Rari** is a high-performance React Server Components framework powered by a Rust runtime. Built for performance, scalability, and developer experience.

With proper app router support, true server-side rendering, and correct RSC semantics, Rari delivers 12x faster P99 latency and 10.5x higher throughput than Next.js.

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
- **3.8x faster response times** - 0.69ms vs 2.58ms average response
- **10.5x higher throughput** - 20,226 req/sec vs 1,934 req/sec under load
- **12x faster P99 latency** - 4ms vs 48ms under concurrent load
- **68% smaller bundles** - 27.6 KB vs 85.9 KB client JavaScript
- **5.6x faster builds** - 1.64s vs 9.11s production builds

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

**Response Time (Single Request):**
| Metric | Rari | Next.js | Improvement |
|--------|------|---------|-------------|
| **Average** | **0.69ms** | 2.58ms | **3.8x faster** |
| **P95** | 1.15ms | 3.37ms | **2.9x faster** |
| **Bundle Size** | 27.6 KB | 85.9 KB | **68% smaller** |

**Throughput Under Load (50 concurrent connections, 30s):**
| Metric | Rari | Next.js | Improvement |
|--------|------|---------|-------------|
| **Requests/sec** | **20,226** | 1,934 | **10.5x higher** |
| **Avg Latency** | **2.04ms** | 25.25ms | **12.4x faster** |
| **P99 Latency** | **4ms** | 48ms | **12x faster** |
| **Errors** | 0 | 0 | Stable |

**Build Performance:**
| Metric | Rari | Next.js | Improvement |
|--------|------|---------|-------------|
| **Build Time** | **1.64s** | 9.11s | **5.6x faster** |
| **Bundle Size** | 273 KB | 742 KB | **63% smaller** |

All benchmarks are reproducible. See [benchmarks/](https://github.com/rari-build/benchmarks) for methodology and scripts.

## Project Structure

```
my-rari-app/
├── src/
│   ├── app/
│   │   ├── layout.tsx        # Root layout (server component)
│   │   ├── page.tsx          # Home page (server component)
│   │   ├── about/
│   │   │   └── page.tsx      # About page
│   │   └── blog/
│   │       ├── page.tsx      # Blog index
│   │       └── [slug]/
│   │           └── page.tsx  # Dynamic blog post route
│   ├── components/
│   │   └── Counter.tsx       # Client components ('use client')
│   ├── actions/
│   │   └── todo-actions.ts   # Server actions ('use server')
│   └── app/
│       └── globals.css       # Global styles
├── vite.config.ts            # Vite + Rari configuration
└── package.json
```

## Examples

### App Router Layout
```tsx
// src/app/layout.tsx - Server component by default
import type { LayoutProps } from 'rari'

export default function RootLayout({ children }: LayoutProps) {
  return (
    <div className="min-h-screen">
      <nav>
        <a href="/">Home</a>
        <a href="/about">About</a>
      </nav>
      <main>{children}</main>
    </div>
  )
}

export const metadata = {
  title: 'My Rari App',
  description: 'Built with Rari',
}
```

### Server Component (Default)
```tsx
// src/app/page.tsx - Runs on the server, no directive needed
import type { PageProps } from 'rari'

export default async function HomePage({ params, searchParams }: PageProps) {
  const data = await fetch('https://api.example.com/data').then(r => r.json())

  return (
    <div>
      <h1>{data.title}</h1>
      <Counter initialValue={data.count} />
    </div>
  )
}

export const metadata = {
  title: 'Home | My App',
  description: 'Welcome to my app',
}
```

### Dynamic Routes
```tsx
// src/app/blog/[slug]/page.tsx - Dynamic route with params
import type { PageProps } from 'rari'

export default function BlogPostPage({ params }: PageProps<{ slug: string }>) {
  return (
    <article>
      <h1>
        Blog Post:
        {params.slug}
      </h1>
    </article>
  )
}

export async function generateMetadata({ params }: PageProps<{ slug: string }>) {
  return {
    title: `${params.slug} | Blog`,
    description: `Read about ${params.slug}`,
  }
}
```

### Client Component
```tsx
// src/components/Counter.tsx - Interactive component
'use client'

import { useState } from 'react'

export default function Counter({ initialValue }: { initialValue: number }) {
  const [count, setCount] = useState(initialValue)

  return (
    <button onClick={() => setCount(count + 1)}>
      Count: {count}
    </button>
  )
}
```

### Server Actions
```tsx
// src/actions/todo-actions.ts - Server functions for mutations
'use server'

export async function addTodo(formData: FormData) {
  const title = formData.get('title') as string
  await db.todos.create({ title })
  return { success: true }
}

export async function deleteTodo(id: string) {
  await db.todos.delete(id)
  return { success: true }
}
```

### Using Server Actions in Client Components
```tsx
// src/components/TodoForm.tsx
'use client'

import { useActionState } from 'react'
import { addTodo } from '../actions/todo-actions'

export default function TodoForm() {
  const [state, formAction, isPending] = useActionState(addTodo, null)

  return (
    <form action={formAction}>
      <input name="title" required />
      <button disabled={isPending}>
        {isPending ? 'Adding...' : 'Add Todo'}
      </button>
    </form>
  )
}
```

## Configuration

Rari works with zero configuration, but you can customize it:

```typescript
// vite.config.ts
import { rari } from 'rari/vite'
import { defineConfig } from 'vite'

export default defineConfig({
  plugins: [
    rari()
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
