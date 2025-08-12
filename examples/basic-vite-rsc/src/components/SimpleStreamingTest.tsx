'use server'

import process from 'node:process'

async function SimpleStreamingTest() {
  await new Promise(resolve => setTimeout(resolve, 500))

  const timestamp = new Date().toLocaleTimeString()
  const serverInfo = {
    nodeVersion: process.version || 'Unknown',
    platform: process.platform || 'Unknown',
    timestamp,
  }

  return (
    <div
      className="p-5 bg-white border rounded-lg shadow-sm"
      data-component-id="simple-streaming-test"
    >
      <h1 className="text-2xl font-bold text-green-700 mb-4">
        Simple Streaming Test ✅
      </h1>

      <div className="mb-4 p-3 bg-green-50 border border-green-200 rounded">
        <p className="text-green-700">
          This component tests basic async rendering without Suspense
          boundaries. It should work with both old and new streaming
          implementations.
        </p>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        <div className="p-4 bg-blue-100 border border-blue-300 rounded">
          <h3 className="text-lg font-semibold text-blue-800">Server Info</h3>
          <div className="text-sm text-blue-700 space-y-1">
            <p>
              <strong>Node Version:</strong>
              {' '}
              {serverInfo.nodeVersion}
            </p>
            <p>
              <strong>Platform:</strong>
              {' '}
              {serverInfo.platform}
            </p>
            <p>
              <strong>Rendered At:</strong>
              {' '}
              {serverInfo.timestamp}
            </p>
          </div>
        </div>

        <div className="p-4 bg-purple-100 border border-purple-300 rounded">
          <h3 className="text-lg font-semibold text-purple-800">Test Status</h3>
          <div className="text-sm text-purple-700 space-y-1">
            <p>✅ Async rendering: Success</p>
            <p>✅ Server execution: Success</p>
            <p>✅ Component props: Success</p>
            <p>⏱️ Render time: ~500ms</p>
          </div>
        </div>
      </div>

      <div className="mt-6 p-3 bg-gray-50 border border-gray-200 rounded">
        <h3 className="font-semibold text-gray-800 mb-2">
          Streaming Endpoints:
        </h3>
        <div className="text-sm text-gray-600 space-y-1">
          <p>
            •
            {' '}
            <code className="bg-gray-200 px-1 rounded">/api/rsc/stream</code>
            {' '}
            - Original streaming (legacy)
          </p>
          <p>
            •
            {' '}
            <code className="bg-gray-200 px-1 rounded">/api/rsc/stream-v2</code>
            {' '}
            - True streaming (new)
          </p>
        </div>
        <p className="text-xs text-gray-500 mt-2">
          Compare both endpoints to see the difference in streaming behavior.
        </p>
      </div>

      <div className="mt-4 p-3 bg-yellow-50 border border-yellow-200 rounded">
        <p className="text-yellow-700 text-sm">
          <strong>Next Test:</strong>
          {' '}
          Once this works, try the
          SuspenseStreamingTest component to see true Suspense boundary
          streaming in action.
        </p>
      </div>
    </div>
  )
}

export default SimpleStreamingTest
