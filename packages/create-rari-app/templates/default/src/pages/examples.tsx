import type { PageProps } from 'rari/client'
import { Link } from 'rari/client'
import ServerTime from '../components/ServerTime'
import Welcome from '../components/Welcome'

export default function ExamplesPage({ params, searchParams }: PageProps) {
  return (
    <div className="min-h-screen bg-gradient-to-br from-purple-50 to-pink-100 py-8 px-4">
      <div className="max-w-4xl mx-auto">
        <div className="text-center mb-12">
          <h1 className="text-5xl font-bold text-gray-900 mb-4">
            Server
            {' '}
            <span className="bg-gradient-to-r from-purple-600 to-pink-600 bg-clip-text text-transparent">
              Components
            </span>
          </h1>
          <p className="text-xl text-gray-600 mb-8">
            Explore React Server Components running on the Rust runtime
          </p>
          <div className="inline-flex items-center px-4 py-2 bg-purple-100 text-purple-800 rounded-lg">
            <svg
              className="w-5 h-5 mr-2"
              fill="currentColor"
              viewBox="0 0 20 20"
            >
              <path
                fillRule="evenodd"
                d="M11.3 1.046A1 1 0 0112 2v5h4a1 1 0 01.82 1.573l-7 10A1 1 0 018 18v-5H4a1 1 0 01-.82-1.573l7-10a1 1 0 011.12-.38z"
                clipRule="evenodd"
              />
            </svg>
            Powered by Rust runtime
          </div>
        </div>

        <div className="grid gap-8 mb-12">
          <div className="bg-white rounded-xl p-8 shadow-sm">
            <h2 className="text-2xl font-semibold mb-6 text-gray-900">
              🕒 Real-time Server Component
            </h2>
            <p className="text-gray-600 mb-6">
              This component is rendered on the server and shows the current server time.
              Each time you navigate to this page, you'll see the time when the component
              was rendered on the Rust runtime.
            </p>
            <ServerTime />
          </div>

          <div className="bg-white rounded-xl p-8 shadow-sm">
            <h2 className="text-2xl font-semibold mb-6 text-gray-900">
              👋 Welcome Component
            </h2>
            <p className="text-gray-600 mb-6">
              This component demonstrates server-side rendering with dynamic content.
              The performance stats and system information are computed on the server.
            </p>
            <Welcome />
          </div>
        </div>

        <div className="bg-white rounded-xl p-8 shadow-sm border border-gray-200 mb-8">
          <h2 className="text-2xl font-bold text-gray-900 mb-6">
            🔍 How Server Components Work
          </h2>

          <div className="grid gap-6 md:grid-cols-2">
            <div>
              <h3 className="text-lg font-semibold text-gray-900 mb-3">
                Server Execution
              </h3>
              <p className="text-gray-600 mb-4">
                Components marked with
                {' '}
                <code className="bg-gray-100 px-2 py-1 rounded">'use server'</code>
                {' '}
                run entirely on the Rust runtime. They can:
              </p>
              <ul className="space-y-2 text-gray-600">
                <li className="flex items-center">
                  <span className="text-green-500 mr-2">✓</span>
                  Access databases directly
                </li>
                <li className="flex items-center">
                  <span className="text-green-500 mr-2">✓</span>
                  Read from the filesystem
                </li>
                <li className="flex items-center">
                  <span className="text-green-500 mr-2">✓</span>
                  Make server-side API calls
                </li>
                <li className="flex items-center">
                  <span className="text-green-500 mr-2">✓</span>
                  Access environment variables
                </li>
              </ul>
            </div>

            <div>
              <h3 className="text-lg font-semibold text-gray-900 mb-3">
                Performance Benefits
              </h3>
              <div className="space-y-3">
                <div className="p-3 bg-green-50 rounded-lg">
                  <div className="font-semibold text-green-800">Zero Client JS</div>
                  <div className="text-sm text-green-600">
                    Server components don't send any JavaScript to the client
                  </div>
                </div>
                <div className="p-3 bg-blue-50 rounded-lg">
                  <div className="font-semibold text-blue-800">Instant Rendering</div>
                  <div className="text-sm text-blue-600">
                    Rendered in ~3ms on the Rust runtime
                  </div>
                </div>
                <div className="p-3 bg-purple-50 rounded-lg">
                  <div className="font-semibold text-purple-800">Automatic Caching</div>
                  <div className="text-sm text-purple-600">
                    Intelligent caching and invalidation
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>

        <div className="bg-white rounded-xl p-8 shadow-sm border border-gray-200 mb-8">
          <h2 className="text-2xl font-bold text-gray-900 mb-6">
            🛠️ Creating Server Components
          </h2>

          <div className="bg-gray-50 rounded-lg p-6">
            <h3 className="text-lg font-semibold text-gray-900 mb-3">
              Example: ServerTime.tsx
            </h3>
            <pre className="bg-gray-800 text-gray-100 p-4 rounded-lg text-sm overflow-x-auto">
              {`'use server'

export default async function ServerTime() {
  // This runs on the Rust runtime!
  const now = new Date()

  // Simulate some async server work
  await new Promise(resolve => setTimeout(resolve, 100))

  return (
    <div className="p-4 bg-blue-50 rounded-lg">
      <h2 className="text-lg font-semibold">Server Time</h2>
      <p>Generated at: {now.toISOString()}</p>
    </div>
  )
}`}
            </pre>
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
                <code className="bg-gray-200 px-2 py-1 rounded">src/pages/examples.tsx</code>
              </div>
              <div className="mb-2">
                <strong>Route:</strong>
                {' '}
                <code className="bg-gray-200 px-2 py-1 rounded">/examples</code>
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
            className="inline-flex items-center px-6 py-3 bg-purple-600 text-white rounded-lg hover:bg-purple-700 transition-colors mr-4"
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
          <Link
            to="/about"
            className="inline-flex items-center px-6 py-3 bg-gray-600 text-white rounded-lg hover:bg-gray-700 transition-colors"
          >
            Learn More About Rari
            <svg
              className="w-5 h-5 ml-2"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M14 5l7 7m0 0l-7 7m7-7H3"
              />
            </svg>
          </Link>
        </div>
      </div>
    </div>
  )
}
