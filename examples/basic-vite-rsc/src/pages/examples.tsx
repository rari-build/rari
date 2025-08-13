import { Link } from 'rari/client'
import ComponentSelector from '../components/ComponentSelector'

function ExamplesPage() {
  const serverComponents = [
    {
      id: 'ServerWithClient',
      name: 'Server + Client Test',
    },
    {
      id: 'EnvTestComponent',
      name: 'Environment Test',
    },
    { id: 'ShoppingList', name: 'Shopping List' },
    { id: 'FetchExample', name: 'Fetch Example' },
    { id: 'WhatsHot', name: 'What\'s Hot' },
    { id: 'TestComponent', name: 'Test Component' },
    { id: 'Markdown', name: 'Markdown' },
    { id: 'StressTest', name: 'StressTest' },
    { id: 'SimpleStreamingTest', name: 'Simple Streaming Test' },
    { id: 'SuspenseStreamingTest', name: 'Suspense Streaming Test' },
    { id: 'StreamingVerificationTest', name: 'Streaming Verification Test' },
  ]

  return (
    <div className="min-h-screen bg-gradient-to-br from-blue-50 to-indigo-100 py-8 px-4">
      <div className="max-w-4xl mx-auto">
        <div className="mb-8">
          <Link
            to="/"
            className="inline-flex items-center px-4 py-2 mb-6 text-blue-600 hover:text-blue-800 transition-colors"
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

          <div className="text-center">
            <h1 className="text-4xl font-bold text-gray-900 mb-2">
              <span className="bg-gradient-to-r from-blue-600 to-purple-600 bg-clip-text text-transparent">
                Component Examples
              </span>
            </h1>
            <p className="text-xl text-gray-600 mb-4">
              Explore React Server Components in action
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
              Server Components Active
            </div>
          </div>
        </div>

        <div className="bg-white rounded-xl p-8 shadow-sm border border-gray-200">
          <ComponentSelector serverComponents={serverComponents} />
        </div>

        <div className="mt-8 text-center">
          <p className="text-gray-500 text-sm">
            Select a component above to see React Server Components in action.
            {' '}
            <br />
            Components are rendered on the server and hydrated on the client.
          </p>
        </div>
      </div>
    </div>
  )
}

export default ExamplesPage
