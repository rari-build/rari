import { useState } from 'react'
import ServerTime from '../components/ServerTime'
import Welcome from '../components/Welcome'

export default function HomePage({ params, searchParams }: { params: { [key: string]: string }, searchParams: { [key: string]: string } }) {
  const [count, setCount] = useState(0)

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

        <div className="grid gap-8 md:grid-cols-2 mb-12">
          <div className="bg-white rounded-xl p-8 shadow-sm">
            <h2 className="text-2xl font-semibold mb-4 text-gray-900">
              Client Interaction
            </h2>
            <p className="text-gray-600 mb-6">
              This counter runs in the browser with React's client-side
              interactivity.
            </p>
            <div className="flex items-center gap-4">
              <button
                onClick={() => setCount(count - 1)}
                className="px-4 py-2 bg-red-500 text-white rounded-lg hover:bg-red-600 transition-colors"
                type="button"
              >
                -
              </button>
              <span className="text-2xl font-bold text-gray-900 min-w-[2rem] text-center">
                {count}
              </span>
              <button
                onClick={() => setCount(count + 1)}
                className="px-4 py-2 bg-green-500 text-white rounded-lg hover:bg-green-600 transition-colors"
                type="button"
              >
                +
              </button>
            </div>
          </div>

          <div className="space-y-6">
            <Welcome />
            <ServerTime />
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

          <a
            href="/about"
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
          </a>
        </div>

        <div className="bg-white rounded-xl p-8 shadow-sm border border-gray-200 mb-8">
          <h2 className="text-2xl font-bold text-gray-900 mb-6">
            File-Based Routing
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
              </div>
            </div>

            <div>
              <h3 className="text-lg font-semibold text-gray-900 mb-3">
                Current Route
              </h3>
              <div className="p-4 bg-blue-50 rounded-lg">
                <div className="text-sm text-blue-800">
                  <div>
                    Path:
                    {' '}
                    <code className="bg-blue-100 px-2 py-1 rounded">/</code>
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
          </div>
        </div>

        <div className="mt-12 text-center">
          <p className="text-gray-500 text-sm">
            Edit
            {' '}
            <code className="bg-gray-100 px-2 py-1 rounded text-xs">
              src/pages/index.tsx
            </code>
            {' '}
            or create new files in
            {' '}
            <code className="bg-gray-100 px-2 py-1 rounded text-xs">
              src/pages/
            </code>
            {' '}
            to see file-based routing in action!
          </p>
        </div>
      </div>
    </div>
  )
}
