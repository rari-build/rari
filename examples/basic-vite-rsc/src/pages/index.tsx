import type { PageProps } from 'rari/client'
import { Link } from 'rari/client'

export default function HomePage({ params, searchParams }: PageProps) {
  return (
    <div className="min-h-screen bg-gradient-to-br from-blue-50 to-indigo-100 py-8 px-4">
      <div className="max-w-4xl mx-auto">
        <div className="text-center mb-12">
          <h1 className="text-5xl font-bold text-gray-900 mb-4">
            Welcome to
            {' '}
            <span className="bg-gradient-to-r from-blue-600 to-purple-600 bg-clip-text text-transparent">
              Rari
            </span>
          </h1>
          <p className="text-xl text-gray-600 mb-8">
            Runtime Accelerated Rendering Infrastructure with File-Based Routing
          </p>
          <div className="inline-flex items-center px-4 py-2 bg-green-100 text-green-800 rounded-lg">
            <svg
              className="w-5 h-5 mr-2"
              fill="currentColor"
              viewBox="0 0 20 20"
            >
              <path
                fillRule="evenodd"
                d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z"
                clipRule="evenodd"
              />
            </svg>
            File-based routing is active!
          </div>
        </div>

        <div className="grid gap-6 md:grid-cols-2 lg:grid-cols-3 mb-12">
          <div className="bg-white rounded-xl p-6 shadow-sm border border-gray-200">
            <h3 className="text-lg font-semibold text-gray-900 mb-3">
              🏠 Home Page
            </h3>
            <p className="text-gray-600 mb-4">
              You're currently on the home page (pages/index.tsx)
            </p>
            <div className="text-sm text-gray-500">
              Route:
              {' '}
              <code className="bg-gray-100 px-2 py-1 rounded">/</code>
            </div>
          </div>

          <Link
            to="/about"
            className="bg-white rounded-xl p-6 shadow-sm border border-gray-200 hover:shadow-md transition-shadow block"
          >
            <h3 className="text-lg font-semibold text-gray-900 mb-3">
              📄 About Page
            </h3>
            <p className="text-gray-600 mb-4">Learn more about this project</p>
            <div className="text-sm text-gray-500">
              Route:
              {' '}
              <code className="bg-gray-100 px-2 py-1 rounded">/about</code>
            </div>
          </Link>

          <Link
            to="/users/123"
            className="bg-white rounded-xl p-6 shadow-sm border border-gray-200 hover:shadow-md transition-shadow block"
          >
            <h3 className="text-lg font-semibold text-gray-900 mb-3">
              👤 User Profile
            </h3>
            <p className="text-gray-600 mb-4">Dynamic route example</p>
            <div className="text-sm text-gray-500">
              Route:
              {' '}
              <code className="bg-gray-100 px-2 py-1 rounded">/users/[id]</code>
            </div>
          </Link>

          <Link
            to="/examples"
            className="bg-white rounded-xl p-6 shadow-sm border border-gray-200 hover:shadow-md transition-shadow block"
          >
            <h3 className="text-lg font-semibold text-gray-900 mb-3">
              🧩 Components
            </h3>
            <p className="text-gray-600 mb-4">
              Explore React Server Components in action
            </p>
            <div className="text-sm text-gray-500">
              Route:
              {' '}
              <code className="bg-gray-100 px-2 py-1 rounded">/examples</code>
            </div>
          </Link>
        </div>

        <div className="bg-white rounded-xl p-8 shadow-sm border border-gray-200">
          <h2 className="text-2xl font-bold text-gray-900 mb-6">
            File-Based Routing Examples
          </h2>

          <div className="grid gap-4 md:grid-cols-2">
            <div>
              <h3 className="text-lg font-semibold text-gray-900 mb-3">
                Static Routes
              </h3>
              <div className="space-y-2 text-sm">
                <div className="flex justify-between">
                  <code className="text-gray-600">pages/index.tsx</code>
                  <span className="text-gray-400">→</span>
                  <code className="text-blue-600">/</code>
                </div>
                <div className="flex justify-between">
                  <code className="text-gray-600">pages/about.tsx</code>
                  <span className="text-gray-400">→</span>
                  <code className="text-blue-600">/about</code>
                </div>
                <div className="flex justify-between">
                  <code className="text-gray-600">pages/blog/index.tsx</code>
                  <span className="text-gray-400">→</span>
                  <code className="text-blue-600">/blog</code>
                </div>
              </div>
            </div>

            <div>
              <h3 className="text-lg font-semibold text-gray-900 mb-3">
                Dynamic Routes
              </h3>
              <div className="space-y-2 text-sm">
                <div className="flex justify-between">
                  <code className="text-gray-600">pages/users/[id].tsx</code>
                  <span className="text-gray-400">→</span>
                  <code className="text-blue-600">/users/:id</code>
                </div>
                <div className="flex justify-between">
                  <code className="text-gray-600">pages/blog/[slug].tsx</code>
                  <span className="text-gray-400">→</span>
                  <code className="text-blue-600">/blog/:slug</code>
                </div>
                <div className="flex justify-between">
                  <code className="text-gray-600">pages/[...slug].tsx</code>
                  <span className="text-gray-400">→</span>
                  <code className="text-blue-600">/:slug*</code>
                </div>
              </div>
            </div>
          </div>

          <div className="mt-6 p-4 bg-blue-50 rounded-lg">
            <h4 className="font-semibold text-blue-900 mb-2">
              Current Route Info:
            </h4>
            <div className="text-sm text-blue-800">
              <div>
                Path:
                {' '}
                <code className="bg-blue-100 px-2 py-1 rounded">
                  /
                  {Object.keys(params).length
                    ? Object.entries(params)
                        .map(([k, v]) => `${k}=${v}`)
                        .join('&')
                    : ''}
                </code>
              </div>
              {Object.keys(params).length > 0 && (
                <div className="mt-1">
                  Params:
                  {' '}
                  <code className="bg-blue-100 px-2 py-1 rounded">
                    {JSON.stringify(params)}
                  </code>
                </div>
              )}
              {Object.keys(searchParams).length > 0 && (
                <div className="mt-1">
                  Search:
                  {' '}
                  <code className="bg-blue-100 px-2 py-1 rounded">
                    {JSON.stringify(searchParams)}
                  </code>
                </div>
              )}
            </div>
          </div>
        </div>

        <div className="mt-8 text-center">
          <p className="text-gray-500 text-sm">
            Edit pages in
            {' '}
            <code className="bg-gray-100 px-2 py-1 rounded">src/pages/</code>
            {' '}
            to
            see file-based routing in action!
          </p>
        </div>
      </div>
    </div>
  )
}
