import type { PageProps } from 'rari/client'
import { Link } from 'rari/client'

export default function AboutPage({ params, searchParams }: PageProps) {
  return (
    <div className="min-h-screen bg-gradient-to-br from-purple-50 to-pink-100 py-8 px-4">
      <div className="max-w-4xl mx-auto">
        <div className="mb-8">
          <Link
            to="/"
            className="inline-flex items-center text-purple-600 hover:text-purple-800 transition-colors"
          >
            <svg
              className="w-5 h-5 mr-2"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M10 19l-7-7m0 0l7-7m-7 7h18"
              />
            </svg>
            Back to Home
          </Link>
        </div>

        <div className="text-center mb-12">
          <h1 className="text-4xl font-bold text-gray-900 mb-4">
            About
            {' '}
            <span className="bg-gradient-to-r from-purple-600 to-pink-600 bg-clip-text text-transparent">
              Rari
            </span>
          </h1>
          <p className="text-xl text-gray-600">
            Runtime Accelerated Rendering Infrastructure
          </p>
        </div>

        <div className="grid gap-8 md:grid-cols-2 mb-12">
          <div className="bg-white rounded-xl p-8 shadow-sm border border-gray-200">
            <h2 className="text-2xl font-bold text-gray-900 mb-4">
              🚀 What is Rari?
            </h2>
            <p className="text-gray-600 mb-4">
              Rari is a cutting-edge React framework that combines the power of
              React Server Components with file-based routing, delivering
              exceptional performance and developer experience.
            </p>
            <ul className="space-y-2 text-gray-600">
              <li className="flex items-start">
                <span className="text-green-500 mr-2">✓</span>
                React Server Components support
              </li>
              <li className="flex items-start">
                <span className="text-green-500 mr-2">✓</span>
                File-based routing system
              </li>
              <li className="flex items-start">
                <span className="text-green-500 mr-2">✓</span>
                TypeScript-first development
              </li>
              <li className="flex items-start">
                <span className="text-green-500 mr-2">✓</span>
                Zero-config setup
              </li>
            </ul>
          </div>

          <div className="bg-white rounded-xl p-8 shadow-sm border border-gray-200">
            <h2 className="text-2xl font-bold text-gray-900 mb-4">
              ⚡ Performance
            </h2>
            <p className="text-gray-600 mb-4">
              Built for speed from the ground up, Rari delivers exceptional
              performance that outpaces traditional React frameworks.
            </p>
            <div className="space-y-3">
              <div className="flex justify-between items-center">
                <span className="text-gray-600">vs Next.js</span>
                <span className="text-green-600 font-semibold">
                  4.04x faster
                </span>
              </div>
              <div className="flex justify-between items-center">
                <span className="text-gray-600">Response Time</span>
                <span className="text-green-600 font-semibold">4.23ms</span>
              </div>
              <div className="flex justify-between items-center">
                <span className="text-gray-600">Throughput</span>
                <span className="text-green-600 font-semibold">
                  10,586 req/sec
                </span>
              </div>
              <div className="flex justify-between items-center">
                <span className="text-gray-600">Build Speed</span>
                <span className="text-green-600 font-semibold">
                  77.5% faster
                </span>
              </div>
            </div>
          </div>
        </div>

        <div className="bg-white rounded-xl p-8 shadow-sm border border-gray-200 mb-8">
          <h2 className="text-2xl font-bold text-gray-900 mb-6">
            🛠️ Key Features
          </h2>

          <div className="grid gap-6 md:grid-cols-3">
            <div className="text-center">
              <div className="w-12 h-12 bg-blue-100 rounded-lg flex items-center justify-center mx-auto mb-4">
                <svg
                  className="w-6 h-6 text-blue-600"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={2}
                    d="M13 10V3L4 14h7v7l9-11h-7z"
                  />
                </svg>
              </div>
              <h3 className="font-semibold text-gray-900 mb-2">
                Server Components
              </h3>
              <p className="text-sm text-gray-600">
                Render React components on the server for better performance and
                SEO
              </p>
            </div>

            <div className="text-center">
              <div className="w-12 h-12 bg-green-100 rounded-lg flex items-center justify-center mx-auto mb-4">
                <svg
                  className="w-6 h-6 text-green-600"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={2}
                    d="M9 20l-5.447-2.724A1 1 0 013 16.382V5.618a1 1 0 011.447-.894L9 7m0 13l6-3m-6 3V7m6 10l4.553 2.276A1 1 0 0021 18.382V7.618a1 1 0 00-.553-.894L15 4m0 13V4m0 0L9 7"
                  />
                </svg>
              </div>
              <h3 className="font-semibold text-gray-900 mb-2">
                File-based Routing
              </h3>
              <p className="text-sm text-gray-600">
                Automatic route generation based on your file structure
              </p>
            </div>

            <div className="text-center">
              <div className="w-12 h-12 bg-purple-100 rounded-lg flex items-center justify-center mx-auto mb-4">
                <svg
                  className="w-6 h-6 text-purple-600"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={2}
                    d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"
                  />
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={2}
                    d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"
                  />
                </svg>
              </div>
              <h3 className="font-semibold text-gray-900 mb-2">Zero Config</h3>
              <p className="text-sm text-gray-600">
                Get started immediately with sensible defaults and minimal setup
              </p>
            </div>
          </div>
        </div>

        <div className="bg-white rounded-xl p-8 shadow-sm border border-gray-200 mb-8">
          <h2 className="text-2xl font-bold text-gray-900 mb-6">
            🗺️ Routing System
          </h2>

          <div className="grid gap-6 md:grid-cols-2">
            <div>
              <h3 className="text-lg font-semibold text-gray-900 mb-3">
                How it works
              </h3>
              <p className="text-gray-600 mb-4">
                Rari automatically generates routes based on your file structure
                in the
                {' '}
                <code className="bg-gray-100 px-2 py-1 rounded">src/pages</code>
                {' '}
                directory.
              </p>
              <div className="space-y-2 text-sm">
                <div className="p-3 bg-gray-50 rounded-lg">
                  <div className="font-mono text-gray-700 mb-1">
                    pages/index.tsx
                  </div>
                  <div className="text-gray-500">→ Routes to "/"</div>
                </div>
                <div className="p-3 bg-gray-50 rounded-lg">
                  <div className="font-mono text-gray-700 mb-1">
                    pages/about.tsx
                  </div>
                  <div className="text-gray-500">→ Routes to "/about"</div>
                </div>
                <div className="p-3 bg-gray-50 rounded-lg">
                  <div className="font-mono text-gray-700 mb-1">
                    pages/users/[id].tsx
                  </div>
                  <div className="text-gray-500">→ Routes to "/users/:id"</div>
                </div>
              </div>
            </div>

            <div>
              <h3 className="text-lg font-semibold text-gray-900 mb-3">
                Navigation
              </h3>
              <p className="text-gray-600 mb-4">
                Use the built-in Link component for client-side navigation with
                automatic prefetching.
              </p>
              <div className="space-y-3">
                <Link
                  to="/users/123"
                  className="flex items-center p-3 border border-gray-200 rounded-lg hover:bg-gray-50 transition-colors"
                >
                  <span className="text-gray-700">User Profile Example</span>
                  <svg
                    className="w-4 h-4 ml-auto text-gray-400"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                  >
                    <path
                      strokeLinecap="round"
                      strokeLinejoin="round"
                      strokeWidth={2}
                      d="M9 5l7 7-7 7"
                    />
                  </svg>
                </Link>
              </div>
            </div>
          </div>
        </div>

        <div className="bg-gradient-to-r from-blue-50 to-purple-50 rounded-xl p-8 border border-gray-200">
          <h2 className="text-2xl font-bold text-gray-900 mb-4">
            🎯 Current Route Information
          </h2>
          <div className="grid gap-4 md:grid-cols-3">
            <div className="bg-white p-4 rounded-lg">
              <div className="text-sm text-gray-500 mb-1">File Path</div>
              <code className="text-sm font-mono text-gray-800">
                pages/about.tsx
              </code>
            </div>
            <div className="bg-white p-4 rounded-lg">
              <div className="text-sm text-gray-500 mb-1">Route Pattern</div>
              <code className="text-sm font-mono text-gray-800">/about</code>
            </div>
            <div className="bg-white p-4 rounded-lg">
              <div className="text-sm text-gray-500 mb-1">Route Type</div>
              <span className="text-sm font-medium text-green-600">Static</span>
            </div>
          </div>

          {Object.keys(params).length > 0 && (
            <div className="mt-4 bg-white p-4 rounded-lg">
              <div className="text-sm text-gray-500 mb-1">Route Parameters</div>
              <code className="text-sm font-mono text-gray-800">
                {JSON.stringify(params, null, 2)}
              </code>
            </div>
          )}

          {Object.keys(searchParams).length > 0 && (
            <div className="mt-4 bg-white p-4 rounded-lg">
              <div className="text-sm text-gray-500 mb-1">
                Search Parameters
              </div>
              <code className="text-sm font-mono text-gray-800">
                {JSON.stringify(searchParams, null, 2)}
              </code>
            </div>
          )}
        </div>
      </div>
    </div>
  )
}
