# Getting Started with Rari

This guide will walk you through creating your first Rari application from scratch. You'll learn the basics of React Server Components with Rari's high-performance Rust runtime.

## Prerequisites

Before you begin, ensure you have the following installed:

- **Node.js 20+**
- **npm, pnpm, or yarn**
- **A modern code editor**

## Installation

### Option 1: Create New Project (Recommended)

The fastest way to get started is with our project generator:

```bash
npm create rari-app@latest my-rari-app
cd my-rari-app
# Dependencies are automatically installed
npm run dev
```

Or with other package managers:

```bash
# Using pnpm (recommended)
pnpm create rari-app my-rari-app

# Using yarn
yarn create rari-app my-rari-app

# Using bun
bun create rari-app my-rari-app
```

### Option 2: Add to Existing Vite Project

If you have an existing Vite + React project:

```bash
npm install rari
```

Update your `vite.config.ts`:

```typescript
import { rari } from 'rari'
import { defineConfig } from 'vite'

export default defineConfig({
  plugins: [
    rari(), // Add Rari plugin
  ],
})
```

## Your First Server Component

Create `src/components/ServerTime.tsx`:

```tsx
'use server'

// This is a React Server Component
export default async function ServerTime() {
  // This runs on the server!
  const timestamp = new Date().toISOString()

  // Simulate async work
  await new Promise(resolve => setTimeout(resolve, 100))

  return (
    <div className="p-4 bg-blue-50 rounded-lg">
      <h2 className="text-lg font-semibold">Server Time</h2>
      <p className="text-gray-600">
        Generated on server at:
        {' '}
        {timestamp}
      </p>
    </div>
  )
}
```

Use it in your `src/App.tsx`:

```tsx
import { useState } from 'react'
import ServerTime from './components/ServerTime'

export default function App() {
  const [count, setCount] = useState(0)

  return (
    <div className="min-h-screen bg-gray-100 py-8">
      <div className="max-w-2xl mx-auto">
        <h1 className="text-3xl font-bold text-center mb-8">
          My First Rari App
        </h1>

        {/* Server Component */}
        <ServerTime />

        {/* Client Component */}
        <div className="mt-8 p-4 bg-white rounded-lg shadow">
          <h2 className="text-lg font-semibold mb-4">Client Interaction</h2>
          <button
            onClick={() => setCount(count + 1)}
            type="button"
            className="px-4 py-2 bg-blue-500 text-white rounded hover:bg-blue-600"
          >
            Count:
            {' '}
            {count}
          </button>
        </div>
      </div>
    </div>
  )
}
```

## Using NPM Packages

One of Rari's superpowers is seamless NPM package integration. Let's add markdown support:

```bash
npm install markdown-it
```

Create `src/components/MarkdownPost.tsx`:

```tsx
'use server'

import MarkdownIt from 'markdown-it'

interface MarkdownPostProps {
  content: string
  title: string
}

export default async function MarkdownPost({ content, title }: MarkdownPostProps) {
  // Process markdown on the server
  const md = MarkdownIt({
    html: true,
    breaks: false,
    linkify: true,
  })
  const htmlContent = md.render(content)

  return (
    <article className="prose max-w-none">
      <h1>{title}</h1>
      {/* eslint-disable-next-line react-dom/no-dangerously-set-innerhtml */}
      <div dangerouslySetInnerHTML={{ __html: htmlContent }} />
    </article>
  )
}
```

Use it in your app:

```tsx
import MarkdownPost from './components/MarkdownPost'

const blogPost = `
# Welcome to Rari!

This markdown is processed **on the server** using the \`markdown-it\` package.

- Fast server-side rendering
- Universal NPM package support
- Zero configuration required
`

export default function App() {
  return (
    <div className="max-w-4xl mx-auto py-8 px-4">
      <MarkdownPost
        title="My Blog Post"
        content={blogPost}
      />
    </div>
  )
}
```

## Development Workflow

### Start Development Server

```bash
npm run dev
```

Your app will be available at `http://localhost:5173` (Vite default) with:
- **Hot module reloading** for instant updates
- **Error overlay** for debugging
- **Fast Rust runtime** powering server components
- **TypeScript support** out of the box

### Build for Production

```bash
npm run build
```

This creates an optimized production build with:
- **Automatic code splitting**
- **Asset optimization**
- **Server bundle generation**

### Start Production Server

```bash
npm run start
```

Runs your optimized app in production mode.

## Project Structure

A typical Rari project (created with `create-rari-app`) looks like this:

```
my-rari-app/
├── src/
│   ├── components/          # Reusable React components
│   │   ├── ServerTime.tsx   # Server components ('use server')
│   │   └── Welcome.tsx      # Client components
│   ├── pages/               # File-based routing
│   │   ├── index.tsx        # Home page (/)
│   │   ├── about.tsx        # About page (/about)
│   │   ├── blog/
│   │   │   ├── index.tsx    # Blog index (/blog)
│   │   │   └── [slug].tsx   # Blog post (/blog/[slug])
│   │   └── users/
│   │       └── [id].tsx     # User profile (/users/[id])
│   ├── styles/              # CSS files
│   │   └── index.css        # Main stylesheet with Tailwind
│   ├── App.tsx              # Root component with RouterProvider
│   ├── main.tsx             # Client entry point
│   └── vite-env.d.ts       # Vite types
├── public/                  # Static assets (optional)
├── index.html              # HTML template
├── package.json            # Dependencies and scripts
├── vite.config.ts          # Vite + Rari config
├── tsconfig.json           # TypeScript project config
├── tsconfig.app.json       # App-specific TypeScript config
├── tsconfig.node.json      # Node.js TypeScript config
└── .gitignore              # Git ignore rules
```

## File-Based Routing

Rari uses file-based routing where your file structure automatically determines your application routes. This makes organizing your application intuitive and scalable.

### Basic Routes

Static routes are created by adding files to your `src/pages/` directory:

- `src/pages/index.tsx` → `/`
- `src/pages/about.tsx` → `/about`
- `src/pages/contact.tsx` → `/contact`

```tsx
// src/pages/about.tsx
export default function AboutPage() {
  return (
    <div>
      <h1>About Us</h1>
      <p>Welcome to our amazing application!</p>
    </div>
  )
}
```

### Dynamic Routes

Use square brackets for dynamic route parameters:

- `src/pages/users/[id].tsx` → `/users/123`
- `src/pages/blog/[slug].tsx` → `/blog/my-post`

```tsx
// src/pages/users/[id].tsx
import type { PageProps } from 'rari/client'

export default function UserPage({ params }: PageProps) {
  const { id } = params

  return (
    <div>
      <h1>User Profile</h1>
      <p>
        User ID:
        {id}
      </p>
    </div>
  )
}
```

### Nested Routes

Create nested routes using directories:

- `src/pages/dashboard/index.tsx` → `/dashboard`
- `src/pages/dashboard/analytics.tsx` → `/dashboard/analytics`
- `src/pages/dashboard/settings.tsx` → `/dashboard/settings`

### Catch-All Routes

Use spread syntax for catch-all routes:

- `src/pages/docs/[...slug].tsx` → `/docs/getting-started/installation`

```tsx
// src/pages/docs/[...slug].tsx
import type { PageProps } from 'rari/client'

export default function DocsPage({ params }: PageProps) {
  const { slug } = params // slug is an array: ['getting-started', 'installation']

  return (
    <div>
      <h1>Documentation</h1>
      <p>
        Path:
        {slug.join('/')}
      </p>
    </div>
  )
}
```

### Navigation

Use Rari's navigation components for client-side routing:

```tsx
import { Link, NavLink } from 'rari/client'

export default function Navigation() {
  return (
    <nav>
      <Link to="/">Home</Link>
      <Link to="/about">About</Link>

      {/* NavLink provides active state */}
      <NavLink
        to="/blog"
        activeClassName="text-blue-500"
        className="text-gray-600"
      >
        Blog
      </NavLink>
    </nav>
  )
}
```

### Router Setup

Your main `App.tsx` uses the `RouterProvider`:

```tsx
import { RouterProvider, useRouter } from 'rari/client'
import { routes } from '../.rari/routes' // Auto-generated

// eslint-disable-next-line react-refresh/only-export-components
function App() {
  return (
    <RouterProvider routes={routes}>
      <Routes />
    </RouterProvider>
  )
}

// eslint-disable-next-line react-refresh/only-export-components
function Routes() {
  const { currentRoute } = useRouter()

  if (!currentRoute) {
    return <div>Page not found</div>
  }

  const Component = currentRoute.route.component
  const { params, searchParams } = currentRoute

  return (
    <Component
      params={params}
      searchParams={searchParams}
    />
  )
}
```

## Server vs Client Components

### Server Components (Default)
- Run on the server during rendering
- Can use async/await and server-only APIs
- Cannot use browser APIs or event handlers
- Automatically serialized for the client
- Use `'use server'` directive for clarity

```tsx
'use server'

// Server Component (default)
export default async function ServerComponent() {
  const data = await fetch('https://api.example.com/data')
  const result = await data.json()
  return <div>{result.message}</div>
}
```

### Client Components
- Run in the browser
- Can use hooks, event handlers, browser APIs
- Cannot use server-only APIs

```tsx
import { useState } from 'react'

export default function ClientComponent() {
  const [count, setCount] = useState(0)

  return (
    <button
      type="button"
      onClick={() => setCount(count + 1)}
    >
      Clicked
      {' '}
      {count}
      {' '}
      times
    </button>
  )
}
```

## Common Patterns

### Loading Data in Routes

```tsx
'use server'

// src/pages/users/[id].tsx
import type { PageProps } from 'rari/client'

export default async function UserProfile({ params }: PageProps) {
  const { id } = params

  // Fetch data on the server
  const user = await fetch(`https://api.example.com/users/${id}`)
    .then(r => r.json())

  return (
    <div>
      <h1>{user.name}</h1>
      <p>{user.email}</p>
      <p>
        Joined:
        {user.createdAt}
      </p>
    </div>
  )
}
```

### Combining Server and Client

```tsx
'use server'

// Server component that includes client components
import ClientCounter from '../components/ClientCounter'

export default async function Dashboard() {
  const stats = await getServerStats()

  return (
    <div>
      <h1>Dashboard</h1>
      <div>
        Server stats:
        {' '}
        {stats.totalUsers}
      </div>
      <ClientCounter />
      {' '}
      {/* Client component */}
    </div>
  )
}
```

### Programmatic Navigation

```tsx
import { useRouter } from 'rari/client'

export default function LoginForm() {
  const { navigate } = useRouter()

  const handleSubmit = async (formData: FormData) => {
    const success = await login(formData)

    if (success) {
      navigate('/dashboard') // Redirect after login
    }
  }

  return (
    <form action={handleSubmit}>
      <input name="email" type="email" required />
      <input name="password" type="password" required />
      <button type="submit">Login</button>
    </form>
  )
}
```

### Error Handling

```tsx
'use server'

export default async function DataComponent() {
  try {
    const data = await fetchSomeData()
    return <div>{data.content}</div>
  }
  catch (error) {
    return (
      <div className="error">
        <h2>Something went wrong</h2>
        <p>{error instanceof Error ? error.message : 'Unknown error'}</p>
      </div>
    )
  }
}
```
