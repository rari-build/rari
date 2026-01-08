import type { PageProps } from 'rari/client'

export default async function HomePage({ params, searchParams }: PageProps) {
  return (
    <div className="space-y-8">
      <div className="bg-white rounded-lg shadow-sm border border-gray-200 p-8 md:p-12">
        <div className="flex items-center gap-3 mb-4">
          <h1 className="text-4xl md:text-5xl font-bold text-gray-900">
            Welcome to Rari App Router
          </h1>
          <span className="text-3xl">🚀</span>
        </div>

        <p className="text-lg text-gray-600 mb-6 max-w-3xl leading-relaxed">
          A demonstration of the Next.js-style app router implementation in
          Rari. Explore server-side rendering, React Server Components, and
          modern routing patterns.
        </p>

        <div className="flex flex-wrap gap-3">
          <span className="inline-flex items-center px-3 py-1 text-xs font-medium text-green-700 bg-green-50 border border-green-200 rounded-full">
            ✓ Production Ready
          </span>
          <span className="inline-flex items-center px-3 py-1 text-xs font-medium text-blue-700 bg-blue-50 border border-blue-200 rounded-full">
            ✓ Type Safe
          </span>
          <span className="inline-flex items-center px-3 py-1 text-xs font-medium text-purple-700 bg-purple-50 border border-purple-200 rounded-full">
            ✓ Server First
          </span>
        </div>
      </div>

      <div>
        <h2 className="text-2xl font-bold text-gray-900 mb-4">Core Features</h2>
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
          <div className="bg-white rounded-lg shadow-sm border border-gray-200 p-6 hover:shadow-md transition-shadow">
            <div className="text-2xl mb-3">📁</div>
            <h3 className="font-semibold text-gray-900 mb-2">
              File-based Routing
            </h3>
            <p className="text-sm text-gray-600">
              Automatic routing based on your file structure
            </p>
          </div>

          <div className="bg-white rounded-lg shadow-sm border border-gray-200 p-6 hover:shadow-md transition-shadow">
            <div className="text-2xl mb-3">🎨</div>
            <h3 className="font-semibold text-gray-900 mb-2">Nested Layouts</h3>
            <p className="text-sm text-gray-600">
              Share UI across routes with layout composition
            </p>
          </div>

          <div className="bg-white rounded-lg shadow-sm border border-gray-200 p-6 hover:shadow-md transition-shadow">
            <div className="text-2xl mb-3">⚡</div>
            <h3 className="font-semibold text-gray-900 mb-2">
              Server Rendering
            </h3>
            <p className="text-sm text-gray-600">
              Fast initial page loads with SSR
            </p>
          </div>

          <div className="bg-white rounded-lg shadow-sm border border-gray-200 p-6 hover:shadow-md transition-shadow">
            <div className="text-2xl mb-3">🔄</div>
            <h3 className="font-semibold text-gray-900 mb-2">Server Actions</h3>
            <p className="text-sm text-gray-600">
              Call server functions directly from components
            </p>
          </div>

          <div className="bg-white rounded-lg shadow-sm border border-gray-200 p-6 hover:shadow-md transition-shadow">
            <div className="text-2xl mb-3">🌐</div>
            <h3 className="font-semibold text-gray-900 mb-2">Dynamic Routes</h3>
            <p className="text-sm text-gray-600">
              Handle parameters and catch-all routes
            </p>
          </div>

          <div className="bg-white rounded-lg shadow-sm border border-gray-200 p-6 hover:shadow-md transition-shadow">
            <div className="text-2xl mb-3">📊</div>
            <h3 className="font-semibold text-gray-900 mb-2">Data Fetching</h3>
            <p className="text-sm text-gray-600">
              Server-side data loading with async components
            </p>
          </div>

          <div className="bg-white rounded-lg shadow-sm border border-gray-200 p-6 hover:shadow-md transition-shadow">
            <div className="text-2xl mb-3">⏳</div>
            <h3 className="font-semibold text-gray-900 mb-2">Loading States</h3>
            <p className="text-sm text-gray-600">
              Built-in loading and error boundaries
            </p>
          </div>

          <div className="bg-white rounded-lg shadow-sm border border-gray-200 p-6 hover:shadow-md transition-shadow">
            <div className="text-2xl mb-3">🔍</div>
            <h3 className="font-semibold text-gray-900 mb-2">SEO Optimized</h3>
            <p className="text-sm text-gray-600">
              Metadata generation for search engines
            </p>
          </div>
        </div>
      </div>

      <div>
        <h2 className="text-2xl font-bold text-gray-900 mb-4">
          Try the Examples
        </h2>
        <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
          <a
            href="/actions"
            className="group block bg-white rounded-lg shadow-sm border border-gray-200 p-6 no-underline hover:shadow-md hover:border-gray-300 transition-all"
          >
            <div className="flex items-center gap-3 mb-3">
              <span className="text-2xl">⚡</span>
              <h3 className="text-lg font-semibold text-gray-900 group-hover:text-gray-700">
                Server Actions
              </h3>
            </div>
            <p className="text-sm text-gray-600 mb-3">
              Interactive todo app demonstrating React Server Actions with
              useActionState
            </p>
            <span className="text-sm font-medium text-indigo-600 group-hover:text-indigo-700">
              View demo →
            </span>
          </a>

          <a
            href="/server-data"
            className="group block bg-white rounded-lg shadow-sm border border-gray-200 p-6 no-underline hover:shadow-md hover:border-gray-300 transition-all"
          >
            <div className="flex items-center gap-3 mb-3">
              <span className="text-2xl">📊</span>
              <h3 className="text-lg font-semibold text-gray-900 group-hover:text-gray-700">
                Server Data
              </h3>
            </div>
            <p className="text-sm text-gray-600 mb-3">
              Server-side data fetching with async server components
            </p>
            <span className="text-sm font-medium text-indigo-600 group-hover:text-indigo-700">
              View demo →
            </span>
          </a>

          <a
            href="/interactive"
            className="group block bg-white rounded-lg shadow-sm border border-gray-200 p-6 no-underline hover:shadow-md hover:border-gray-300 transition-all"
          >
            <div className="flex items-center gap-3 mb-3">
              <span className="text-2xl">🎮</span>
              <h3 className="text-lg font-semibold text-gray-900 group-hover:text-gray-700">
                Interactive
              </h3>
            </div>
            <p className="text-sm text-gray-600 mb-3">
              Client components working seamlessly with React Server Components
            </p>
            <span className="text-sm font-medium text-indigo-600 group-hover:text-indigo-700">
              View demo →
            </span>
          </a>

          <a
            href="/test-image"
            className="group block bg-white rounded-lg shadow-sm border border-gray-200 p-6 no-underline hover:shadow-md hover:border-gray-300 transition-all"
          >
            <div className="flex items-center gap-3 mb-3">
              <span className="text-2xl">🖼️</span>
              <h3 className="text-lg font-semibold text-gray-900 group-hover:text-gray-700">
                Image Component
              </h3>
            </div>
            <p className="text-sm text-gray-600 mb-3">
              High-performance image optimization powered by Rust
            </p>
            <span className="text-sm font-medium text-indigo-600 group-hover:text-indigo-700">
              View demo →
            </span>
          </a>
        </div>
      </div>

      <details className="bg-white rounded-lg shadow-sm border border-gray-200 p-6">
        <summary className="font-semibold text-gray-900 cursor-pointer select-none">
          Debug Information
        </summary>
        <div className="mt-4 p-4 bg-gray-50 rounded border border-gray-200">
          <pre className="text-xs text-gray-700 overflow-auto">
            {JSON.stringify({ params, searchParams }, null, 2)}
          </pre>
        </div>
      </details>
    </div>
  )
}

export const metadata = {
  title: 'Home | Rari App Router',
  description: 'Welcome to the Rari app router example',
}
