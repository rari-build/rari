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
import { rari, rariRouter } from 'rari/vite'
import { defineConfig } from 'vite'

export default defineConfig({
  plugins: [
    rari(), // Core Rari plugin
    rariRouter(), // App router support
  ],
})
```

## Your First App Router Page

With the app router, your application structure is based on the `src/app/` directory. Let's create your first page.

Create `src/app/layout.tsx` (root layout):

```tsx
import type { LayoutProps } from 'rari/client'

export default function RootLayout({ children }: LayoutProps) {
  return (
    <div className="min-h-screen bg-gray-100">
      <nav className="bg-white shadow-sm p-4">
        <div className="max-w-7xl mx-auto flex gap-4">
          <a href="/" className="text-blue-600 hover:text-blue-800">Home</a>
          <a href="/about" className="text-blue-600 hover:text-blue-800">About</a>
        </div>
      </nav>
      <main className="max-w-7xl mx-auto py-8 px-4">
        {children}
      </main>
    </div>
  )
}

export const metadata = {
  title: 'My Rari App',
  description: 'Built with Rari',
}
```

Create `src/app/page.tsx` (home page):

```tsx
import type { PageProps } from 'rari/client'
import Counter from '../components/Counter'

// This is a React Server Component - runs on the server!
export default async function HomePage({ params, searchParams }: PageProps) {
  const timestamp = new Date().toISOString()

  // Simulate async work
  await new Promise(resolve => setTimeout(resolve, 100))

  return (
    <div className="space-y-8">
      <h1 className="text-3xl font-bold">Welcome to Rari</h1>

      {/* Server-rendered content */}
      <div className="p-4 bg-blue-50 rounded-lg">
        <h2 className="text-lg font-semibold">Server Time</h2>
        <p className="text-gray-600">
          Generated on server at:
          {' '}
          {timestamp}
        </p>
      </div>

      {/* Client Component */}
      <Counter />
    </div>
  )
}

export const metadata = {
  title: 'Home | My Rari App',
  description: 'Welcome to my Rari application',
}
```

Create `src/components/Counter.tsx` (client component):

```tsx
'use client'

import { useState } from 'react'

export default function Counter() {
  const [count, setCount] = useState(0)

  return (
    <div className="p-4 bg-white rounded-lg shadow">
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

Use it in a page:

```tsx
// src/app/blog/page.tsx
import MarkdownPost from '../../components/MarkdownPost'

const blogPost = `
# Welcome to Rari!

This markdown is processed **on the server** using the \`markdown-it\` package.

- Fast server-side rendering
- Universal NPM package support
- Zero configuration required
`

export default function BlogPage() {
  return (
    <div className="max-w-4xl mx-auto">
      <MarkdownPost
        title="My Blog Post"
        content={blogPost}
      />
    </div>
  )
}

export const metadata = {
  title: 'Blog | My Rari App',
  description: 'Read our latest posts',
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
│   ├── app/                 # App router directory
│   │   ├── layout.tsx       # Root layout (wraps all pages)
│   │   ├── page.tsx         # Home page (/)
│   │   ├── globals.css      # Global styles
│   │   ├── about/
│   │   │   └── page.tsx     # About page (/about)
│   │   ├── blog/
│   │   │   ├── page.tsx     # Blog index (/blog)
│   │   │   └── [slug]/
│   │   │       └── page.tsx # Blog post (/blog/[slug])
│   │   └── users/
│   │       └── [id]/
│   │           └── page.tsx # User profile (/users/[id])
│   ├── components/          # Reusable React components
│   │   └── Counter.tsx      # Client components ('use client')
│   ├── actions/             # Server actions
│   │   └── todo-actions.ts  # Server functions ('use server')
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

## App Router

Rari uses the app router pattern where your file structure in `src/app/` automatically determines your application routes. This makes organizing your application intuitive and scalable.

### Basic Routes

Routes are created by adding `page.tsx` files in the `src/app/` directory:

- `src/app/page.tsx` → `/`
- `src/app/about/page.tsx` → `/about`
- `src/app/contact/page.tsx` → `/contact`

```tsx
// src/app/about/page.tsx
import type { PageProps } from 'rari/client'

export default function AboutPage({ params, searchParams }: PageProps) {
  return (
    <div>
      <h1>About Us</h1>
      <p>Welcome to our amazing application!</p>
    </div>
  )
}

export const metadata = {
  title: 'About | My App',
  description: 'Learn more about us',
}
```

### Dynamic Routes

Use square brackets for dynamic route parameters:

- `src/app/users/[id]/page.tsx` → `/users/123`
- `src/app/blog/[slug]/page.tsx` → `/blog/my-post`

```tsx
// src/app/users/[id]/page.tsx
import type { PageProps } from 'rari/client'

export default function UserPage({ params }: PageProps<{ id: string }>) {
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

export async function generateMetadata({ params }: PageProps<{ id: string }>) {
  return {
    title: `User ${params.id} | My App`,
    description: `Profile page for user ${params.id}`,
  }
}
```

### Layouts

Layouts wrap pages and can be nested. They persist across route changes:

```tsx
// src/app/layout.tsx - Root layout (required)
import type { LayoutProps } from 'rari/client'

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
  title: 'My App',
  description: 'Built with Rari',
}
```

```tsx
// src/app/dashboard/layout.tsx - Nested layout
import type { LayoutProps } from 'rari/client'

export default function DashboardLayout({ children }: LayoutProps) {
  return (
    <div className="flex">
      <aside className="w-64 bg-gray-100">
        <nav>
          <a href="/dashboard">Overview</a>
          <a href="/dashboard/analytics">Analytics</a>
          <a href="/dashboard/settings">Settings</a>
        </nav>
      </aside>
      <div className="flex-1">{children}</div>
    </div>
  )
}
```

### Navigation

Use standard HTML links for navigation - the app router handles client-side navigation automatically:

```tsx
export default function Navigation() {
  return (
    <nav>
      <a href="/">Home</a>
      <a href="/about">About</a>
      <a href="/blog">Blog</a>
    </nav>
  )
}
```

## Server vs Client Components

### Server Components (Default)
- Run on the server during rendering
- Can use async/await and server-only APIs
- Cannot use browser APIs or event handlers
- Automatically serialized for the client

```tsx
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
- Use `'use client'` directive to mark explicitly

```tsx
'use client'

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

### Server Actions
- Use `'use server'` directive for server functions that can be called from client components
- Place in `src/actions/` directory for organization

```tsx
// src/actions/user-actions.ts
'use server'

export async function createUser(formData: FormData) {
  const name = formData.get('name') as string
  const email = formData.get('email') as string

  const user = await database.users.create({ name, email })
  return { success: true, user }
}

export async function deleteUser(id: string) {
  await database.users.delete(id)
  return { success: true }
}
```

Use server actions in client components with `useActionState`:

```tsx
// src/components/UserForm.tsx
'use client'

import { useActionState } from 'react'
import { createUser } from '../actions/user-actions'

export default function UserForm() {
  const [state, formAction, isPending] = useActionState(createUser, null)

  return (
    <form action={formAction}>
      <input name="name" required />
      <input name="email" type="email" required />
      <button type="button" disabled={isPending}>
        {isPending ? 'Creating...' : 'Create User'}
      </button>
      {state?.success && <p>User created successfully!</p>}
    </form>
  )
}
```

## Common Patterns

### Loading Data in Pages

```tsx
// src/app/users/[id]/page.tsx
import type { PageProps } from 'rari/client'

export default async function UserProfile({ params }: PageProps<{ id: string }>) {
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

export async function generateMetadata({ params }: PageProps<{ id: string }>) {
  const user = await fetch(`https://api.example.com/users/${params.id}`)
    .then(r => r.json())

  return {
    title: `${user.name} | Users`,
    description: `Profile page for ${user.name}`,
  }
}
```

### Combining Server and Client

```tsx
// src/app/dashboard/page.tsx - Server component that includes client components
import type { PageProps } from 'rari/client'
import ClientCounter from '../../components/ClientCounter'

export default async function Dashboard({ params, searchParams }: PageProps) {
  const stats = await getServerStats()

  return (
    <div>
      <h1>Dashboard</h1>
      <div>
        Server stats:
        {stats.totalUsers}
      </div>
      <ClientCounter />
      {' '}
      {/* Client component */}
    </div>
  )
}

export const metadata = {
  title: 'Dashboard | My App',
  description: 'Your dashboard',
}
```

### Error Handling

```tsx
export default async function DataComponent() {
  let data
  let error

  try {
    data = await fetchSomeData()
  }
  catch (err) {
    error = err
  }

  if (error) {
    return (
      <div className="error">
        <h2>Something went wrong</h2>
        <p>{error instanceof Error ? error.message : 'Unknown error'}</p>
      </div>
    )
  }

  return <div>{data.content}</div>
}
```
