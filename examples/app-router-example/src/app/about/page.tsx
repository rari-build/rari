import type { Metadata } from 'rari'

export default function AboutPage() {
  return (
    <div className="space-y-8">
      <div className="bg-white rounded-lg shadow-sm border border-gray-200 p-8 md:p-12">
        <h1 className="text-4xl font-bold text-gray-900 mb-4">
          About This Example
        </h1>
        <p className="text-lg text-gray-600 leading-relaxed max-w-3xl">
          This example demonstrates the rari app router, a server-first routing
          system inspired by Next.js App Router. It showcases modern React
          patterns including Server Components, Server Actions, and streaming.
        </p>
      </div>

      <div>
        <h2 className="text-2xl font-bold text-gray-900 mb-4">Key Features</h2>
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
          <div className="bg-white rounded-lg shadow-sm border border-gray-200 p-6 hover:shadow-md transition-shadow">
            <div className="w-12 h-12 bg-indigo-50 rounded-lg flex items-center justify-center mb-4">
              <span className="text-2xl">⚛️</span>
            </div>
            <h3 className="text-lg font-semibold text-gray-900 mb-2">
              Server Components
            </h3>
            <p className="text-sm text-gray-600 leading-relaxed">
              React Server Components rendered on the server for better
              performance and smaller bundle sizes
            </p>
          </div>

          <div className="bg-white rounded-lg shadow-sm border border-gray-200 p-6 hover:shadow-md transition-shadow">
            <div className="w-12 h-12 bg-green-50 rounded-lg flex items-center justify-center mb-4">
              <span className="text-2xl">🎨</span>
            </div>
            <h3 className="text-lg font-semibold text-gray-900 mb-2">
              Nested Layouts
            </h3>
            <p className="text-sm text-gray-600 leading-relaxed">
              Share UI across routes with nested layout composition and maintain
              state during navigation
            </p>
          </div>

          <div className="bg-white rounded-lg shadow-sm border border-gray-200 p-6 hover:shadow-md transition-shadow">
            <div className="w-12 h-12 bg-blue-50 rounded-lg flex items-center justify-center mb-4">
              <span className="text-2xl">📁</span>
            </div>
            <h3 className="text-lg font-semibold text-gray-900 mb-2">
              File-Based Routing
            </h3>
            <p className="text-sm text-gray-600 leading-relaxed">
              Automatic routing based on your file structure with support for
              dynamic segments
            </p>
          </div>

          <div className="bg-white rounded-lg shadow-sm border border-gray-200 p-6 hover:shadow-md transition-shadow">
            <div className="w-12 h-12 bg-purple-50 rounded-lg flex items-center justify-center mb-4">
              <span className="text-2xl">🔄</span>
            </div>
            <h3 className="text-lg font-semibold text-gray-900 mb-2">
              Server Actions
            </h3>
            <p className="text-sm text-gray-600 leading-relaxed">
              Call server functions directly from components with automatic
              serialization and type safety
            </p>
          </div>

          <div className="bg-white rounded-lg shadow-sm border border-gray-200 p-6 hover:shadow-md transition-shadow">
            <div className="w-12 h-12 bg-amber-50 rounded-lg flex items-center justify-center mb-4">
              <span className="text-2xl">⚡</span>
            </div>
            <h3 className="text-lg font-semibold text-gray-900 mb-2">
              Streaming SSR
            </h3>
            <p className="text-sm text-gray-600 leading-relaxed">
              Progressive rendering with Suspense boundaries for faster time to
              first byte
            </p>
          </div>

          <div className="bg-white rounded-lg shadow-sm border border-gray-200 p-6 hover:shadow-md transition-shadow">
            <div className="w-12 h-12 bg-rose-50 rounded-lg flex items-center justify-center mb-4">
              <span className="text-2xl">🔍</span>
            </div>
            <h3 className="text-lg font-semibold text-gray-900 mb-2">
              SEO Friendly
            </h3>
            <p className="text-sm text-gray-600 leading-relaxed">
              Built-in metadata generation for optimal search engine
              optimization
            </p>
          </div>
        </div>
      </div>

      <div className="bg-white rounded-lg shadow-sm border border-gray-200 p-8">
        <h2 className="text-2xl font-bold text-gray-900 mb-6">
          Architecture Overview
        </h2>
        <div className="space-y-6">
          <div className="flex gap-4">
            <div className="shrink-0 w-8 h-8 bg-indigo-100 text-indigo-600 rounded-full flex items-center justify-center font-semibold text-sm">
              1
            </div>
            <div>
              <h3 className="font-semibold text-gray-900 mb-1">
                File-System Based Router
              </h3>
              <p className="text-sm text-gray-600 leading-relaxed">
                The router automatically generates routes based on the file
                structure in your
                {' '}
                <code className="px-2 py-0.5 bg-gray-100 rounded text-xs font-mono">
                  src/app
                </code>
                {' '}
                directory. Special files like
                {' '}
                <code className="px-2 py-0.5 bg-gray-100 rounded text-xs font-mono">
                  page.tsx
                </code>
                ,
                {' '}
                <code className="px-2 py-0.5 bg-gray-100 rounded text-xs font-mono">
                  layout.tsx
                </code>
                , and
                {' '}
                <code className="px-2 py-0.5 bg-gray-100 rounded text-xs font-mono">
                  loading.tsx
                </code>
                {' '}
                define route behavior.
              </p>
            </div>
          </div>

          <div className="flex gap-4">
            <div className="shrink-0 w-8 h-8 bg-indigo-100 text-indigo-600 rounded-full flex items-center justify-center font-semibold text-sm">
              2
            </div>
            <div>
              <h3 className="font-semibold text-gray-900 mb-1">
                Server-First Rendering
              </h3>
              <p className="text-sm text-gray-600 leading-relaxed">
                By default, all components are React Server Components, rendered
                on the server. Client components are explicitly marked with
                {' '}
                <code className="px-2 py-0.5 bg-gray-100 rounded text-xs font-mono">
                  'use client'
                </code>
                {' '}
                directive.
              </p>
            </div>
          </div>

          <div className="flex gap-4">
            <div className="shrink-0 w-8 h-8 bg-indigo-100 text-indigo-600 rounded-full flex items-center justify-center font-semibold text-sm">
              3
            </div>
            <div>
              <h3 className="font-semibold text-gray-900 mb-1">
                Async Data Fetching
              </h3>
              <p className="text-sm text-gray-600 leading-relaxed">
                Server components can be async functions, allowing you to fetch
                data directly in your component without additional APIs or
                hooks.
              </p>
            </div>
          </div>

          <div className="flex gap-4">
            <div className="shrink-0 w-8 h-8 bg-indigo-100 text-indigo-600 rounded-full flex items-center justify-center font-semibold text-sm">
              4
            </div>
            <div>
              <h3 className="font-semibold text-gray-900 mb-1">
                Progressive Enhancement
              </h3>
              <p className="text-sm text-gray-600 leading-relaxed">
                Forms work without JavaScript and are enhanced with client-side
                interactivity when available. Server Actions provide the
                server-side logic.
              </p>
            </div>
          </div>
        </div>
      </div>

      <div className="bg-white rounded-lg shadow-sm border border-gray-200 p-8">
        <h2 className="text-2xl font-bold text-gray-900 mb-4">
          Technology Stack
        </h2>
        <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
          <div className="text-center p-4 bg-gray-50 rounded-lg border border-gray-200">
            <div className="text-3xl mb-2">⚛️</div>
            <div className="font-semibold text-gray-900 text-sm">React 19</div>
            <div className="text-xs text-gray-500 mt-1">Server Components</div>
          </div>
          <div className="text-center p-4 bg-gray-50 rounded-lg border border-gray-200">
            <div className="text-3xl mb-2">📘</div>
            <div className="font-semibold text-gray-900 text-sm">
              TypeScript
            </div>
            <div className="text-xs text-gray-500 mt-1">Type Safety</div>
          </div>
          <div className="text-center p-4 bg-gray-50 rounded-lg border border-gray-200">
            <div className="text-3xl mb-2">⚡</div>
            <div className="font-semibold text-gray-900 text-sm">Vite</div>
            <div className="text-xs text-gray-500 mt-1">Build Tool</div>
          </div>
          <div className="text-center p-4 bg-gray-50 rounded-lg border border-gray-200">
            <div className="text-3xl mb-2">🎨</div>
            <div className="font-semibold text-gray-900 text-sm">Tailwind</div>
            <div className="text-xs text-gray-500 mt-1">Styling</div>
          </div>
        </div>
      </div>
    </div>
  )
}

export const metadata: Metadata = {
  title: 'About | rari App Router',
  description: 'Learn about the rari app router implementation',
}
