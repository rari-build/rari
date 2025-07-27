import type { PageProps } from 'rari/client'
import { Link } from 'rari/client'

export default function AboutPage({ params, searchParams }: PageProps) {
  return (
    <div className="min-h-screen bg-gradient-to-br from-green-50 to-blue-100 py-8 px-4">
      <div className="max-w-4xl mx-auto">
        <div className="text-center mb-12">
          <h1 className="text-5xl font-bold text-gray-900 mb-4">
            About
            {' '}
            <span className="bg-gradient-to-r from-green-600 to-blue-600 bg-clip-text text-transparent">
              Rari
            </span>
          </h1>
          <p className="text-xl text-gray-600 mb-8">
            Learn more about the Runtime Accelerated Rendering Infrastructure
          </p>
          <div className="inline-flex items-center px-4 py-2 bg-blue-100 text-blue-800 rounded-lg">
            <svg
              className="w-5 h-5 mr-2"
              fill="currentColor"
              viewBox="0 0 20 20"
            >
              <path
                fillRule="evenodd"
                d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7-4a1 1 0 11-2 0 1 1 0 012 0zM9 9a1 1 0 000 2v3a1 1 0 001 1h1a1 1 0 100-2v-3a1 1 0 00-1-1H9z"
                clipRule="evenodd"
              />
            </svg>
            Static route: /about
          </div>
        </div>

        <div className="grid gap-8 md:grid-cols-2 mb-12">
          <div className="bg-white rounded-xl p-8 shadow-sm">
            <h2 className="text-2xl font-semibold mb-4 text-gray-900">
              🚀 What is Rari?
            </h2>
            <p className="text-gray-600 mb-4">
              Rari is a next-generation React framework powered by a custom-built
              Rust runtime. It provides performance-optimized server-side rendering with
              seamless client-side hydration.
            </p>
            <ul className="space-y-2 text-gray-600">
              <li className="flex items-center">
                <span className="text-green-500 mr-2">✓</span>
                React Server Components with Rust runtime
              </li>
              <li className="flex items-center">
                <span className="text-green-500 mr-2">✓</span>
                4x faster than Next.js under load
              </li>
              <li className="flex items-center">
                <span className="text-green-500 mr-2">✓</span>
                Zero-config setup
              </li>
              <li className="flex items-center">
                <span className="text-green-500 mr-2">✓</span>
                Universal NPM package support
              </li>
            </ul>
          </div>

          <div className="bg-white rounded-xl p-8 shadow-sm">
            <h2 className="text-2xl font-semibold mb-4 text-gray-900">
              ⚡ Performance Benefits
            </h2>
            <div className="space-y-4">
              <div className="p-4 bg-green-50 rounded-lg">
                <div className="text-lg font-semibold text-green-800">
                  4.04x Faster Under Load
                </div>
                <div className="text-sm text-green-600">
                  4.23ms vs Next.js 17.11ms
                </div>
              </div>
              <div className="p-4 bg-blue-50 rounded-lg">
                <div className="text-lg font-semibold text-blue-800">
                  3.74x More Throughput
                </div>
                <div className="text-sm text-blue-600">
                  10,586 req/s vs Next.js 2,832 req/s
                </div>
              </div>
              <div className="p-4 bg-purple-50 rounded-lg">
                <div className="text-lg font-semibold text-purple-800">
                  46% Smaller Bundles
                </div>
                <div className="text-sm text-purple-600">
                  400KB vs Next.js 742KB
                </div>
              </div>
            </div>
          </div>
        </div>

        <div className="bg-white rounded-xl p-8 shadow-sm border border-gray-200 mb-8">
          <h2 className="text-2xl font-bold text-gray-900 mb-6">
            Key Features
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
                High-performance RSC
              </h3>
              <p className="text-sm text-gray-600">
                React Server Components powered by Rust runtime
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
                    d="M9.663 17h4.673M12 3v1m6.364 1.636l-.707.707M21 12h-1M4 12H3m3.343-5.657l-.707-.707m2.828 9.9a5 5 0 117.072 0l-.548.547A3.374 3.374 0 0014 18.469V19a2 2 0 11-4 0v-.531c0-.895-.356-1.754-.988-2.386l-.548-.547z"
                  />
                </svg>
              </div>
              <h3 className="font-semibent text-gray-900 mb-2">
                Zero Config
              </h3>
              <p className="text-sm text-gray-600">
                Works out of the box with pre-built binaries
              </p>
            </div>
          </div>
        </div>

        <div className="bg-white rounded-xl p-8 shadow-sm border border-gray-200 mb-8">
          <h2 className="text-2xl font-bold text-gray-900 mb-6">
            Route Information
          </h2>
          <div className="p-4 bg-gray-50 rounded-lg">
            <div className="text-sm text-gray-700">
              <div className="mb-2">
                <strong>File:</strong>
                {' '}
                <code className="bg-gray-200 px-2 py-1 rounded">src/pages/about.tsx</code>
              </div>
              <div className="mb-2">
                <strong>Route:</strong>
                {' '}
                <code className="bg-gray-200 px-2 py-1 rounded">/about</code>
              </div>
              {Object.keys(params).length > 0 && (
                <div className="mb-2">
                  <strong>Params:</strong>
                  {' '}
                  <code className="bg-gray-200 px-2 py-1 rounded">
                    {JSON.stringify(params)}
                  </code>
                </div>
              )}
              {Object.keys(searchParams).length > 0 && (
                <div>
                  <strong>Search Params:</strong>
                  {' '}
                  <code className="bg-gray-200 px-2 py-1 rounded">
                    {JSON.stringify(searchParams)}
                  </code>
                </div>
              )}
            </div>
          </div>
        </div>

        <div className="text-center">
          <Link
            to="/"
            className="inline-flex items-center px-6 py-3 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors"
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
          <div className="mt-4">
            <Link
              to="/examples"
              className="text-blue-600 hover:text-blue-700 transition-colors"
            >
              Explore Server Components →
            </Link>
          </div>
        </div>
      </div>
    </div>
  )
}
